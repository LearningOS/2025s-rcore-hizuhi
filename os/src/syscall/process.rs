//! Process management syscalls
use crate::task::{change_program_brk, current_user_token, exit_current_and_run_next, get_task_calls, suspend_current_and_run_next, task_mmap, task_munmap};
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
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");

    let user_satp = current_user_token();
    if !is_va_writable(user_satp, ts as *mut u8, 16) {
        return -1;
    }

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

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    println!("[DEBUG] sys_trace: request={}, id={:#x}, data={:#x}", trace_request, id, data);
    let user_satp: usize = current_user_token();
    match trace_request {
        0 => {
            if !is_va_readable(user_satp, id as *const u8, 1) {
                return -1;
            }
            let buffers = translated_byte_buffer(user_satp, id as *const u8, 1);
            if buffers.is_empty() {
                return -1;
            }
            buffers[0][0] as isize
        },
        1 => {
            if !is_va_writable(user_satp, id as *const u8, 1) {
                return -1;
            }
            let mut buffers = translated_byte_buffer(user_satp, id as *const u8, 1);
            if buffers.is_empty() {
                return -1;
            }
            buffers[0][0] = data as u8;
            0
        },
        2 => {
            get_task_calls(id)
        },
        _ => -1
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    println!("[DEBUG] sys_mmap: start={:#x}, len={}, port={:#x}", start, len, port);
    task_mmap(start, len, port)
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    println!("[DEBUG] sys_munmap: start={:#x}, len={}", start, len);
    task_munmap(start, len)
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
