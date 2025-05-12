//!Stdin & Stdout
use super::File;
use crate::mm::{translated_refmut, UserBuffer};
use crate::sbi::console_getchar;
use crate::task::{current_user_token, suspend_current_and_run_next};
use crate::fs::{Stat, StatMode};

/// stdin file for getting chars from console
pub struct Stdin;

/// stdout file for putting chars to console
pub struct Stdout;

impl File for Stdin {
    fn readable(&self) -> bool {
        true
    }
    fn writable(&self) -> bool {
        false
    }
    fn read(&self, mut user_buf: UserBuffer) -> usize {
        assert_eq!(user_buf.len(), 1);
        // busy loop
        let mut c: usize;
        loop {
            c = console_getchar();
            if c == 0 {
                suspend_current_and_run_next();
                continue;
            } else {
                break;
            }
        }
        let ch = c as u8;
        unsafe {
            user_buf.buffers[0].as_mut_ptr().write_volatile(ch);
        }
        1
    }
    fn write(&self, _user_buf: UserBuffer) -> usize {
        panic!("Cannot write to stdin!");
    }
    fn stat(&self, stat: &mut Stat) {
        trace!("stat stdin");
        let token = current_user_token();
        let stat = translated_refmut(token, stat);
        stat.dev = 0;
        stat.ino = 0u64;
        stat.mode = StatMode::NULL;
        stat.nlink = 0;
    }
}

impl File for Stdout {
    fn readable(&self) -> bool {
        false
    }
    fn writable(&self) -> bool {
        true
    }
    fn read(&self, _user_buf: UserBuffer) -> usize {
        panic!("Cannot read from stdout!");
    }
    fn write(&self, user_buf: UserBuffer) -> usize {
        for buffer in user_buf.buffers.iter() {
            print!("{}", core::str::from_utf8(*buffer).unwrap());
        }
        user_buf.len()
    }
    fn stat(&self, stat: &mut Stat) {
        debug!("stat stdout");
        stat.dev = 0;
        stat.ino = 0u64;
        stat.mode = StatMode::NULL;
        stat.nlink = 0;
    }
}
