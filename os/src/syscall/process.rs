//! Process management syscalls

use crate::{
    config::MAX_SYSCALL_NUM,
    mm::{translated_refmut, MapPermission, VirtAddr},
    task::*,
    task::{
        change_program_brk, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus,
    },
    timer::{get_time_us, get_time_ms},
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
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let ts = translated_refmut(current_user_token(), ts);

    *ts = TimeVal {
        sec: get_time_ms() / 1000,
        usec: get_time_us(),
    }; 
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info");
    // status
    let status = get_task_status();

    // syscall_times
    let syscall_times = get_syscall_times();

    // time using
    let time = get_time_using();

    let ti = translated_refmut(current_user_token(), ti);
    *ti = TaskInfo {
        status,
        syscall_times,
        time,
    };

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

    let start_addr = VirtAddr::from(_start);
    if start_addr.page_offset() != 0 {
        return -1;
    }

    let end_addr = VirtAddr::from(_start + _len);
    let mut flags: MapPermission = match _port {
        1 => MapPermission::R,
        2 => MapPermission::W,
        3 => MapPermission::R | MapPermission::W,
        4 => MapPermission::X,
        5 => MapPermission::R | MapPermission::X,
        6 => MapPermission::W | MapPermission::X,
        7 => MapPermission::R | MapPermission::W | MapPermission::X,
        _ => panic!("should not reach here"),
    };
    flags |= MapPermission::U;

    return mmap(start_addr, end_addr, flags);
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap");
    if _len == 0 {
        return -1;
    }

    if _len % 4096 != 0 {
        return -1;
    }

    let start_addr = VirtAddr::from(_start);
    if start_addr.page_offset() != 0 {
        return -1;
    }

    let end_addr = VirtAddr::from(_start + _len);
    return munmap(start_addr, end_addr);
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
