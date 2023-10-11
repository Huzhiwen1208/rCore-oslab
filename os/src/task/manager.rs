//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::mm::{MapPermission, VirtAddr};
use crate::sync::UPSafeCell;
use crate::task::current_task;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }

    /// mmap
    pub fn mmap(&mut self, start_va: VirtAddr, end_va: VirtAddr, prot: MapPermission) -> isize {
        let task = current_task().unwrap();

        // should use (*task)!!!! not task
        let mut inner = (*task).inner_exclusive_access();

        for vaddr in start_va.0..end_va.0 {
            let pte = inner.memory_set.translate(VirtAddr::from(vaddr).floor());
            if let Some(pte) = pte {
                if pte.is_valid() {
                    error!("pte exists: Vaddr{}", vaddr);
                    return -1;
                }
            }
        }
        inner.memory_set.insert_framed_area(start_va, end_va, prot);

        for vaddr in start_va.0..end_va.0 {
            let pte = inner.memory_set.translate(VirtAddr::from(vaddr).floor());
            if let Some(pte) = pte {
                if !pte.is_valid() {
                    error!("pte not exists: Vaddr{}", vaddr);
                    return -1;
                }
            } else {
                error!("pte not exists: Vaddr{}", vaddr);
                return -1;
            }
        }

        0
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}

/// mmap
pub fn mmap(start_va: VirtAddr, end_va: VirtAddr, prot: MapPermission) -> isize {
    TASK_MANAGER.exclusive_access().mmap(start_va, end_va, prot)
}
