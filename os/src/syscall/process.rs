//! Process management syscalls

use crate::{
    config::MAX_SYSCALL_NUM,
    mm::{write_time_val, write_task_info, VirtAddr, PTEFlags, mmap},
    task::*,
    task::{
        change_program_brk, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus,
    },
    timer::get_time_us,
};

/// time val
#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    /// sec
    pub sec: usize,
    /// usec
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
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
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let sec = us / 1000000;
    let usec = us % 1000000;
    let tz: TimeVal = TimeVal { sec, usec };

    write_time_val(current_user_token(), _ts as usize, tz);
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info");
    // status
    let status = get_task_status();

    // syscall_times
    let syscall_times = get_syscall_times();

    // time using
    let time_using = get_time_using();

    let ti = TaskInfo{
        status,
        syscall_times,
        time: time_using,
    };

    write_task_info(current_user_token(), _ti as usize, ti);
    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap");
    if _len == 0 {
        return -1;
    }
    if _port & !0x7 != 0 || _port & 0x7 == 0 {
        return -1;
    }

    if _len % 4096 != 0 {
        return -1;
    }
    
    let vaddr = VirtAddr::from(_start);
    if vaddr.page_offset() != 0 {
        return -1;
    }

    let vpn = vaddr.floor();
    let mut flags = match _port {
        1 => PTEFlags::R,
        2 => PTEFlags::W,
        3 => PTEFlags::R | PTEFlags::W,
        4 => PTEFlags::X,
        5 => PTEFlags::R | PTEFlags::X,
        6 => PTEFlags::W | PTEFlags::X,
        7 => PTEFlags::R | PTEFlags::W | PTEFlags::X,
        _ => panic!("should not reach here"),
    };
    flags |= PTEFlags::U;

    let token = current_user_token();
    return mmap(token, vpn, flags);
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
