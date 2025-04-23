//! Process management syscalls
use crate::task::{change_program_brk, exit_current_and_run_next, suspend_current_and_run_next, current_user_token};
use crate::mm::{translated_byte_buffer, is_va_readable, is_va_writable};
use crate::timer::get_time_us;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
/// 已知虚拟地址和时间，将时间写入虚拟地址
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let _us = get_time_us();
    // let time_val = TimeVal {
    //     sec: us / 1_000_000,
    //     usec: us % 1_000_000,
    // };
    
    // let user_satp = current_user_token();
    // let page_table = get_

    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    let user_satp = current_user_token();
    match trace_request {
        0 => {
            if !is_va_readable(user_satp, id as *const u8, 1) {
                return -1;
            }
            let buffers = translated_byte_buffer(user_satp, id as *const u8, 1);
            buffers[0][0] as isize
        },
        1 => {
            if !is_va_writable(user_satp, id as *const u8, 1) {
                return -1;
            }
            let mut buffers = translated_byte_buffer(user_satp, id as *const u8, 1);
            buffers[0][0] = data as u8;
            0
        },
        2 => {
            let buffers = translated_byte_buffer(user_satp, id as *const u8, 1);
            buffers[0][0] as isize
        },
        _ => panic!("Invalid trace request"),
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    -1
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    -1
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
