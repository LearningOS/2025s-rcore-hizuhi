//! Types related to task management
use super::TaskContext;
use crate::config::MAX_SYSCALL_NUM;

/// The task control block (TCB) of a task.
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    /// 系统调用计数
    pub task_calls: [SyscallInfo; MAX_SYSCALL_NUM], //暂时使用列表实现
}

/// 系统调用信息
#[derive(Copy, Clone)]
pub struct SyscallInfo {
    id: usize,
    times: usize
}

impl SyscallInfo {
    /// 初始化系统调用信息为0
    pub fn zero_init() -> Self {
        Self {
            id: 0,
            times: 0,
        }
    }
    /// 根据id初始化系统调用信息
    pub fn init_with_id(id: usize) -> Self {
        Self {
            id,
            times: 0,
        }
    }

    /// 获取系统调用ID
    pub fn get_id(&self) -> usize {
        self.id
    }

    /// 获取系统调用次数
    pub fn get_times(&self) -> usize {
        self.times
    }

    /// 增加系统调用次数
    pub fn add_time(&mut self) {
        self.times += 1;
    }
}

/// The status of a task
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}
