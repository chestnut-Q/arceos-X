use axtask::PBGTaskInfo;
use alloc::vec::Vec;

pub fn ax_sleep_until(deadline: crate::time::AxTimeValue) {
    #[cfg(feature = "multitask")]
    axtask::sleep_until(deadline);
    #[cfg(not(feature = "multitask"))]
    axhal::time::busy_wait_until(deadline);
}

pub fn ax_yield_now() {
    #[cfg(feature = "multitask")]
    axtask::yield_now();
    #[cfg(not(feature = "multitask"))]
    if cfg!(feature = "irq") {
        axhal::arch::wait_for_irqs();
    } else {
        core::hint::spin_loop();
    }
}

pub fn ax_exit(_exit_code: i32) -> ! {
    #[cfg(feature = "multitask")]
    axtask::exit(_exit_code);
    #[cfg(not(feature = "multitask"))]
    axhal::misc::terminate();
}

pub fn ax_open_profile(file_name: &str) -> bool {
    axtask::open_profile(file_name)
}

pub fn ax_open_pbg(file_name: &str, task_infos: &mut Vec<PBGTaskInfo>) {
    axtask::open_pbg(file_name, task_infos);
}

pub fn ax_close_profile() {
    axtask::close_profile();
}

cfg_task! {
    use core::time::Duration;

    /// A handle to a task.
    pub struct AxTaskHandle {
        inner: axtask::AxTaskRef,
        id: u64,
    }

    impl AxTaskHandle {
        /// Returns the task ID.
        pub fn id(&self) -> u64 {
            self.id
        }
    }

    /// A handle to a wait queue.
    ///
    /// A wait queue is used to store sleeping tasks waiting for a certain event
    /// to happen.
    pub struct AxWaitQueueHandle(axtask::WaitQueue);

    impl AxWaitQueueHandle {
        /// Creates a new empty wait queue.
        pub const fn new() -> Self {
            Self(axtask::WaitQueue::new())
        }
    }

    pub fn ax_current_task_id() -> u64 {
        axtask::current().id().as_u64()
    }

    pub fn ax_spawn<F>(
        f: F, 
        name: alloc::string::String, 
        stack_size: usize,
        _nice: isize,
        _runtime: usize,
        _period: usize
    ) -> AxTaskHandle
    where
        F: FnOnce() + Send + 'static,
    {
cfg_if::cfg_if! {
    if #[cfg(feature = "sched_cfs")] {
        let inner = axtask::spawn(f, name, stack_size, _nice);
    } else if #[cfg(feature = "sched_rms")] {
        let inner = axtask::spawn(f, name, stack_size, _runtime, _period);
    } else {
        let inner = axtask::spawn(f, name, stack_size);
    }
}
        AxTaskHandle {
            id: inner.id().as_u64(),
            inner,
        }
    }

    pub fn ax_wait_for_exit(task: AxTaskHandle) -> Option<i32> {
        task.inner.join()
    }

    pub fn ax_set_current_priority(prio: isize) -> crate::AxResult {
        if axtask::set_priority(prio) {
            Ok(())
        } else {
            axerrno::ax_err!(
                BadState,
                "ax_set_current_priority: failed to set task priority"
            )
        }
    }

    pub fn ax_wait_queue_wait(
        wq: &AxWaitQueueHandle,
        until_condition: impl Fn() -> bool,
        timeout: Option<Duration>,
    ) -> bool {
        #[cfg(feature = "irq")]
        if let Some(dur) = timeout {
            return wq.0.wait_timeout_until(dur, until_condition);
        }

        if timeout.is_some() {
            axlog::warn!("ax_wait_queue_wait: the `timeout` argument is ignored without the `irq` feature");
        }
        wq.0.wait_until(until_condition);
        false
    }

    pub fn ax_wait_queue_wake(wq: &AxWaitQueueHandle, count: u32) {
        if count == u32::MAX {
            wq.0.notify_all(true);
        } else {
            for _ in 0..count {
                wq.0.notify_one(true);
            }
        }
    }

    pub fn ax_wait_timeout(wq: &AxWaitQueueHandle, timeout: core::time::Duration) -> bool {
        axlog::info!("ax_wait_timeout: timeout = {:?}", timeout);
        #[cfg(feature = "irq")] 
        return wq.0.wait_timeout(timeout);
        #[allow(unreachable_code)]
        false
    }
}
