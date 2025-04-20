//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the operating system.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.

mod context;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::config::{APP_SIZE_LIMIT, MAX_APP_NUM, MAX_SYSCALL_NUM};
use crate::loader::{get_num_app, init_app_cx, get_base_i};
use crate::sync::UPSafeCell;
use lazy_static::*;
use switch::__switch;
pub use task::{TaskControlBlock, TaskStatus, SyscallInfo};
use crate::syscall::SYSCALL_IDS;

pub use context::TaskContext;

/// The task manager, where all the tasks are managed.
///
/// Functions implemented on `TaskManager` deals with all task state transitions
/// and task context switching. For convenience, you can find wrappers around it
/// in the module level.
///
/// Most of `TaskManager` are hidden behind the field `inner`, to defer
/// borrowing checks to runtime. You can see examples on how to use `inner` in
/// existing functions on `TaskManager`.
pub struct TaskManager {
    /// total number of tasks
    num_app: usize,
    /// use inner value to get mutable access
    inner: UPSafeCell<TaskManagerInner>,
}

/// Inner of Task Manager
pub struct TaskManagerInner {
    /// task list
    tasks: [TaskControlBlock; MAX_APP_NUM],
    /// id of current `Running` task
    current_task: usize,
}

lazy_static! {
    /// Global variable: TASK_MANAGER
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        let mut tasks = [TaskControlBlock {
            task_cx: TaskContext::zero_init(),
            task_status: TaskStatus::UnInit,
            task_calls: [SyscallInfo::zero_init(); MAX_SYSCALL_NUM],
        }; MAX_APP_NUM];
        for (i, task) in tasks.iter_mut().enumerate() {
            task.task_cx = TaskContext::goto_restore(init_app_cx(i)); // 初始任务上下文中返回地址是__restore， 栈指针是内核栈的栈顶。内核栈中的内容是指向程序入口的指令地址
            task.task_status = TaskStatus::Ready;
            for (index, &syscall_id) in SYSCALL_IDS.iter().enumerate() {
                task.task_calls[index] = SyscallInfo::init_with_id(syscall_id);
            }
        }
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                })
            },
        }
    };
}

impl TaskManager {
    /// Run the first task in task list.
    ///
    /// Generally, the first task in task list is an idle task (we call it zero process later).
    /// But in ch3, we load apps statically, so the first task is a real app.
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut TaskContext, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    /// Change the status of current `Running` task into `Ready`.
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Ready;
    }

    /// Change the status of current `Running` task into `Exited`.
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

    /// Find next task to run and return task id.
    ///
    /// In this case, we only return the first `Ready` task in task list.
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// Switch current `Running` task to the task we have found,
    /// or there is no `Ready` task and we can exit with all applications completed
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_cx_ptr: *mut TaskContext = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // go back to user mode
        } else {
            panic!("All applications completed!");
        }
    }

    /// 读取地址上的数据
    pub fn read_task_data(&self, addr: usize) -> isize {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        let current_task_base_addr = get_base_i(current);
        drop(inner);
        if addr > APP_SIZE_LIMIT {
            return -1;
        }
        unsafe {
            *((current_task_base_addr + addr) as *const u8) as isize
        }
    }

    /// 写入地址上的数据
    pub fn write_task_data(&self, addr: usize, data: usize) -> isize {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        let current_task_base_addr = get_base_i(current);
        drop(inner);
        if data > APP_SIZE_LIMIT {
            return -1;
        }
        unsafe {
            *((current_task_base_addr + addr) as *mut u8) = data as u8;
        }
        0
    }

    /// get call times of current task
    pub fn get_task_calls(&self, id: usize) -> isize {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        let current_task_calls: [SyscallInfo; 5] = inner.tasks[current].task_calls;
        drop(inner);
        for task_calls in current_task_calls{
            if task_calls.get_id() == id {
                return task_calls.get_times() as isize;
            }
        }
        -1
    }

    /// 增加当前任务的系统调用计数
    pub fn add_task_call_times(&self, syscall_id: usize) -> isize {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        let mut ret = -1;
        // 查找对应的系统调用并增加计数
        for task_call in &mut inner.tasks[current].task_calls {
            if task_call.get_id() == syscall_id {
                task_call.add_time();
                ret = 0;
                break;
            }
        }
        drop(inner);
        ret
    }

}

/// 读取地址上的数据
pub fn read_task_data(addr: usize) -> isize {
    TASK_MANAGER.read_task_data(addr)
}

/// 写入地址上的数据
pub fn write_task_data(addr: usize, data: usize) -> isize {
    TASK_MANAGER.write_task_data(addr, data)
}

/// 获取当前任务的系统调用计数
pub fn get_task_calls(id: usize) -> isize {
    TASK_MANAGER.get_task_calls(id)
}

/// 增加当前任务的系统调用计数
pub fn add_task_call_times(syscall_id: usize) -> isize {
    TASK_MANAGER.add_task_call_times(syscall_id)
}

/// Run the first task in task list.
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// Switch current `Running` task to the task we have found,
/// or there is no `Ready` task and we can exit with all applications completed
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// Change the status of current `Running` task into `Ready`.
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// Change the status of current `Running` task into `Exited`.
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}