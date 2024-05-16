//! Process management syscalls
use crate::{
    config::MAX_SYSCALL_NUM,
    mm::{MapPermission, VPNRange, VirtAddr},
    task::{
        change_program_brk, exit_current_and_run_next, suspend_current_and_run_next,
        use_current_task_inner, TaskStatus,
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
    trace!("kernel: sys_task_info");
    let ptyaddr=VirtAddr(ti as usize).find_phys_addr_user_space();
    let mut taskinfo=TaskInfo{
        status: TaskStatus::UnInit,
        syscall_times: [0; MAX_SYSCALL_NUM],
        time: 0,
    };
    use_current_task_inner(|inner|{
        taskinfo=TaskInfo{
            status: inner.task_status,
            syscall_times: inner.syscall_times,
            time: get_time_ms()-inner.start_time,
        };
    });
    match ptyaddr{
        None=>-1,
        Some(ptyaddr)=>{
            unsafe{
                *(ptyaddr.0 as *mut TaskInfo)=taskinfo;
            }
            0
        }
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap");
    let start_va: VirtAddr = start.into();
    if !start_va.aligned() {
        return -1;
    }
    let end_va: VirtAddr = VirtAddr::from(start + len).ceil().into();
    if port & !0b111 != 0 || port & 0b111 == 0 {
        return -1;
    }
    let port = PortFlag::from_bits_truncate(port);
    let mut res = 0;
    use_current_task_inner(|inner| {
        if inner
            .memory_set
            .find_area_include_range(VPNRange::new(start_va.floor().into(), end_va.ceil().into()))
            .is_some()
        {
            res = -1;
            return;
        }

        inner
            .memory_set
            .insert_framed_area(start_va, end_va, port.into())
    });
    res
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");
    let start_va: VirtAddr = start.into();
    if !start_va.aligned() {
        return -1;
    }
    let end_va: VirtAddr = VirtAddr::from(start + len).ceil().into();
    let mut res = 0;
    use_current_task_inner(|inner| {
        res = inner
            .memory_set
            .unmap_area_include_range(VPNRange::new(start_va.floor().into(), end_va.ceil().into()))
    });
    res
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