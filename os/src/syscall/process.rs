//! Process management syscalls
use alloc::sync::Arc;

use crate::{
    config::MAX_SYSCALL_NUM,
    loader::get_app_data_by_name,
    mm::{translated_refmut, translated_str, MapPermission, VPNRange, VirtAddr},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus,
    },
    timer::{get_time_ms, get_time_us},
};

bitflags! {
    pub struct PortFlag: usize {
        const R = 1 << 0;
        const W = 1 << 1;
        const X = 1 << 2;
    }
}

impl From<usize> for PortFlag {
    fn from(port: usize) -> Self {
        PortFlag::from_bits_truncate(port)
    }
}

impl From<PortFlag> for MapPermission {
    fn from(port: PortFlag) -> Self {
        MapPermission::from_bits_truncate((port.bits << 1) as u8) | MapPermission::U
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
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
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    trap_cx.x[10] = 0;
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token1 = current_user_token();
    let path = translated_str(token1, path);
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
    trace!(
        "kernel:pid[{}] sys_waitpid [{}]",
        current_task().unwrap().pid.0,
        pid
    );
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        let exit_code = child.inner_exclusive_access().exit_code;
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel:pid[{}] sys_get_time", current_task().unwrap().pid.0);
    let phyaddr = VirtAddr(ts as usize).find_phys_addr_user_space();
    let us = get_time_us();
    match phyaddr {
        None => -1,
        Some(phyaddr) => {
            unsafe {
                *(phyaddr.0 as *mut TimeVal) = TimeVal {
                    sec: us / 1_000_000,
                    usec: us % 1_000_000,
                };
            }
            0
        }
    }
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    trace!(
        "kernel:pid[{}] sys_task_info",
        current_task().unwrap().pid.0
    );
    let phys_addr = VirtAddr(ti as usize).find_phys_addr_user_space();
    let block = current_task().unwrap();
    let inner = block.inner_exclusive_access();
    let task_info = TaskInfo {
        status: inner.task_status,
        syscall_times: inner.syscall_times,
        time: get_time_ms() - inner.start_time,
    };
    drop(inner);
    drop(block);
    match phys_addr {
        None => -1,
        Some(phys_addr) => {
            unsafe {
                *(phys_addr.0 as *mut TaskInfo) = task_info;
            }
            0
        }
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel:pid[{}] sys_mmap", current_task().unwrap().pid.0);
    let start_va: VirtAddr = start.into();
    if !start_va.aligned() {
        return -1;
    }
    let end_va: VirtAddr = VirtAddr::from(start + len).ceil().into();
    if port & !0b111 != 0 || port & 0b111 == 0 {
        return -1;
    }
    let port = PortFlag::from_bits_truncate(port);
    let block = current_task().unwrap();
    let mut inner = block.inner_exclusive_access();
    if inner
        .memory_set
        .find_area_include_range(VPNRange::new(start_va.floor().into(), end_va.ceil().into()))
        .is_some()
    {
        return -1;
    }
    inner
        .memory_set
        .insert_framed_area(start_va, end_va, port.into());
    0
}

pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_munmap", current_task().unwrap().pid.0);
    let start_va: VirtAddr = start.into();
    if !start_va.aligned() {
        return -1;
    }
    let end_va: VirtAddr = VirtAddr::from(start + len).ceil().into();
    let block = current_task().unwrap();
    let mut inner = block.inner_exclusive_access();
    inner
        .memory_set
        .unmap_area_include_range(VPNRange::new(start_va.floor().into(), end_va.ceil().into()))
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
    trace!("kernel:pid[{}] sys_spawn", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let new_task = current_task().unwrap().spawn(data);
        let new_pid = new_task.getpid() as isize;
        add_task(new_task);
        new_pid
    } else {
        -1
    }
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority",
        current_task().unwrap().pid.0
    );
    if prio < 2 {
        return -1;
    }
    let block = current_task().unwrap();
    let mut inner = block.inner_exclusive_access();
    inner.prio = prio as usize;
    prio
}