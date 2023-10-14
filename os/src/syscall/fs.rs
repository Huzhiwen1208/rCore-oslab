//! File and filesystem-related syscalls
use crate::fs::{linkat, open_file, unlinkat, OpenFlags, Stat, StatMode};
use crate::mm::{translated_byte_buffer, translated_refmut, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_write", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_read", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        error!("sys_read: fd >= inner.fd_table.len() fd: {}", fd);
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            error!("sys_read: file is unreadable");
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        error!("sys_read: fd not found, fd: {}", fd);
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    trace!("kernel:pid[{}] sys_open", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    trace!("kernel:pid[{}] sys_close", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

/// YOUR JOB: Implement fstat.
pub fn sys_fstat(fd: usize, st: *mut Stat) -> isize {
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        error!("fd >= inner.fd_table.len()");
        return -1;
    }

    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();

        // get inode_id, mode, links
        let inode_id = file.get_inode_id();
        let stat_mode = match file.get_file_type() {
            0 => StatMode::NULL,
            1 => StatMode::FILE,
            2 => StatMode::DIR,
            _ => panic!("unknown stat mode"),
        };

        let links: usize = file.get_links();
        let new_stat = Stat::new(inode_id as u64, stat_mode, links as u32);

        // get from virtual addr
        // before this, should drop inner for next borrow
        drop(inner);
        let stat = translated_refmut(current_user_token(), st);
        *stat = new_stat;
        return 0;
    }

    error!("fd file not found");
    return -1;
}

/// YOUR JOB: Implement linkat.
pub fn sys_linkat(old_name: *const u8, new_name: *const u8) -> isize {
    // judge if old_name equals new_name
    let old_path = translated_str(current_user_token(), old_name);
    let new_path = translated_str(current_user_token(), new_name);

    if old_path == new_path {
        error!("link same path!");
        return -1;
    }
    return linkat(old_path.as_str(), new_path.as_str());
}

/// YOUR JOB: Implement unlinkat.
pub fn sys_unlinkat(name: *const u8) -> isize {
    let path = translated_str(current_user_token(), name);

    unlinkat(path.as_str())
}
