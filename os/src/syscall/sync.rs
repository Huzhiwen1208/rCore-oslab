use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_list.len() as isize - 1
    }
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.lock();
    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_list.len() - 1
    };

    if process_inner.enable_deadlock_test {
        info!("[create] resource: {{id: {}, count: {}}}", id, res_count);
    }

    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();

    if process_inner.enable_deadlock_test {
        info!(
            "[up] {{tid: {}, sem_id: {}}}",
            current_task()
                .unwrap()
                .inner_exclusive_access()
                .res
                .as_ref()
                .unwrap()
                .tid,
            sem_id
        );
    }

    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.up();
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    if process_inner.enable_deadlock_test {
        let tid = current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid;
        info!("[down] {{tid: {}, sem_id: {}}}", tid, sem_id);

        if tid > 0 && sem_id > 0 {
            assert!(process_inner.allocation.len() <= process_inner.tasks.len() - 1);

            if process_inner.is_allocated() {
                info!(
                    "should write need, tid: {}, value: {:?}",
                    tid, process_inner.need
                );

                let mut changed: Vec<usize>;
                if process_inner.contains_key_need(&tid) {
                    let v = process_inner.get_value_need(&tid);
                    changed = v.clone();
                } else {
                    changed = vec![0; process_inner.semaphore_list.len()];
                }

                changed[sem_id] += 1;
                process_inner.set_value_need(&tid, &changed);

                info!("now need, tid: {}, value: {:?}", tid, process_inner.need);

                if process_inner.is_need_ok() {
                    info!("the last thread entry deadlock test, exec_tid: {}", tid);
                    let r_v = process_inner.deadlock_test();
                    if r_v == -0xdead {
                        warn!("occur deadlock!, should return -0xdead");
                        process_inner.return_value = r_v;

                        info!("release all resoures, wakeup all threads");
                        for item in process_inner.need.values() {
                            for (sem_id, res_count) in item.iter().enumerate() {
                                if *res_count > 0 {
                                    let sem = Arc::clone(
                                        process_inner.semaphore_list[sem_id].as_ref().unwrap(),
                                    );
                                    for _ in 0..*res_count {
                                        sem.up();
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                info!(
                    "should write allocation, tid: {}, value: {:?}",
                    tid, process_inner.allocation
                );

                // should write allocation
                let mut changed: Vec<usize>;
                if process_inner.contains_key_allocation(&tid) {
                    let v = process_inner.get_value_allocation(&tid);
                    changed = v.clone();
                } else {
                    changed = vec![0; process_inner.semaphore_list.len()];
                }

                changed[sem_id] += 1;
                process_inner.set_value_allocation(&tid, &changed);

                info!("now allocation: {:?}", process_inner.allocation);
            }
        }
    }

    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.down();

    let process_inner = process.inner_exclusive_access();
    return process_inner.return_value;
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(enabled: usize) -> isize {
    if enabled == 0 {
        current_process()
            .inner_exclusive_access()
            .enable_deadlock_test = false;
    } else {
        current_process()
            .inner_exclusive_access()
            .enable_deadlock_test = true;
    }
    0
}
