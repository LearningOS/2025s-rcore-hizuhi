//! Process management syscalls
use alloc::sync::Arc;

use crate::{
    config::PAGE_SIZE, loader::get_app_data_by_name, mm::{translated_byte_buffer, translated_refmut, translated_str, MapPermission, VirtAddr}, task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next,
    }, timer::get_time_us, task::TaskStatus
};


#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);

    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0; // 保存在x10，作为__restore的返回值
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!("kernel::pid[{}] sys_waitpid [{}]", current_task().unwrap().pid.0, pid);
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    
    debug!("[kernel] pid[{}] children status:", task.pid.0);
    for child in inner.children.iter() {
        let child_inner = child.inner_exclusive_access();
        let status = match child_inner.task_status {
            TaskStatus::UnInit => "UnInit",
            TaskStatus::Ready => "Ready",
            TaskStatus::Running => "Running", 
            TaskStatus::Zombie => "Zombie",
        };
        debug!("[kernel] \t Child pid[{}] status:{}", child.pid.0, status);
    }

    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_get_time NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );

    let user_satp = current_user_token();

    let mut buffers = translated_byte_buffer(user_satp, ts as *mut u8, 16);
    let us = get_time_us();
    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    match buffers.len() {
        0 => return -1,
        1 => {
            buffers[0][0..8].copy_from_slice(&time_val.sec.to_le_bytes());
            buffers[0][8..16].copy_from_slice(&time_val.usec.to_le_bytes());
        },
        2 => {
            let first_buffer_len = buffers[0].len();
            if first_buffer_len >= 8 {
                buffers[0][0..8].copy_from_slice(&time_val.sec.to_le_bytes());
                buffers[0][8..first_buffer_len].copy_from_slice(&time_val.usec.to_le_bytes()[0..first_buffer_len-8]);
                buffers[1][0..16-first_buffer_len].copy_from_slice(&time_val.usec.to_le_bytes()[first_buffer_len-8..]);
            } else {
                buffers[0][0..first_buffer_len].copy_from_slice(&time_val.sec.to_le_bytes()[0..first_buffer_len]);
                buffers[1][0..8-first_buffer_len].copy_from_slice(&time_val.sec.to_le_bytes()[first_buffer_len..]);
                buffers[1][8-first_buffer_len..16-first_buffer_len].copy_from_slice(&time_val.usec.to_le_bytes());
            }
        },
        _ => return -1,
    }
    0
}

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_mmap",
        current_task().unwrap().pid.0
    );
    if start % PAGE_SIZE != 0 || port & !0x07 != 0 || port & 0x07 == 0{
        return -1;
    }
    let len = (len - 1 + PAGE_SIZE) / PAGE_SIZE * PAGE_SIZE; // 按页对齐（向上取整），如果len小于一个PAGE_SIZE，那么end比start刚好大一页，再经过floor和ceil，分别为start和start+1，其他情况也类似
    let end = start + len;
    let start_va: VirtAddr = VirtAddr::from(start);
    let end_va: VirtAddr = VirtAddr::from(end);

    let mut permission = MapPermission::U;
    if (port & 0x1) != 0 { permission |= MapPermission::R; }
    if (port & 0x2) != 0 { permission |= MapPermission::W; }
    if (port & 0x4) != 0 { permission |= MapPermission::X; }

    let current: Arc<crate::task::TaskControlBlock> = current_task().unwrap();
    let current_tcb = &mut current.inner_exclusive_access();
    debug!("mmap: start_va:{:#x}, end_va:{:#x}", start_va.0, end_va.0);
    current_tcb.memory_set.insert_framed_area(start_va, end_va, permission)
}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_munmap",
        current_task().unwrap().pid.0
    );
    if start % PAGE_SIZE != 0 {
        return -1;
    }
    let start_va = VirtAddr::from(start);
    let start_vpn = start_va.floor();
    let len_vpn = (len - 1 + PAGE_SIZE) / PAGE_SIZE;
    let current: Arc<crate::task::TaskControlBlock> = current_task().unwrap();
    let current_tcb = &mut current.inner_exclusive_access();
    debug!("munmap: start_vpn:{:#x}, len_vpn:{}", start_vpn.0, len_vpn);
    current_tcb.memory_set.remove_area_with_start_vpn_and_len(start_vpn, len_vpn)
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn",
        current_task().unwrap().pid.0
    );

    let current_task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);

    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let new_task = current_task.spawn(data);
        let new_pid = new_task.pid.0;
        let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
        trap_cx.x[10] = 0;
        add_task(new_task);
        debug!("have spawned a new process: pid[{}]", new_pid);
        new_pid as isize
    } else {
        -1
    }
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}