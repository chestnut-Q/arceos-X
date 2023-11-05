use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_init::LazyInit;
use scheduler::BaseScheduler;
use spinlock::SpinNoIrq;
use alloc::vec::Vec;
use crate::get_current_cpu_id;
use lazy_static::lazy_static;
use crate::Manager;
use load_balance_manager::BaseManager;
use crate::AxTask;


use crate::task::{CurrentTask, TaskState};
use crate::{AxTaskRef, Scheduler, TaskInner, WaitQueue};

// TODO: per-CPU
// pub(crate) static RUN_QUEUE: LazyInit<SpinNoIrq<AxRunQueue>> = LazyInit::new();
use array_init::array_init;
lazy_static! {
    pub(crate) static ref RUN_QUEUE: [LazyInit<Arc<SpinNoIrq<AxRunQueue>>>; axconfig::SMP] = array_init(|_| LazyInit::new());
    pub(crate) static ref REAL_RUN_QUEUE: [LazyInit<Arc<SpinNoIrq<Scheduler>>>; axconfig::SMP] = array_init(|_| LazyInit::new());
}

pub(crate) static RUN_MANAGER: LazyInit<SpinNoIrq<Manager>> = LazyInit::new();

// TODO: per-CPU
static EXITED_TASKS: SpinNoIrq<VecDeque<AxTaskRef>> = SpinNoIrq::new(VecDeque::new());

static WAIT_FOR_EXIT: WaitQueue = WaitQueue::new();

#[percpu::def_percpu]
static IDLE_TASK: LazyInit<AxTaskRef> = LazyInit::new();

pub(crate) struct AxRunQueue {
    scheduler: Arc<SpinNoIrq<Scheduler>>,
}

impl AxRunQueue {

cfg_if::cfg_if! {
if #[cfg(feature = "sched_cfs")] {
    pub fn new(_nice: isize) -> SpinNoIrq<Self> {
        let gc_task: Arc<scheduler::CFSTask<TaskInner>> = 
            TaskInner::new(gc_entry, "gc".into(), axconfig::TASK_STACK_SIZE, _nice);
        let mut scheduler = Scheduler::new();
        scheduler.add_task(gc_task);
        SpinNoIrq::new(Self { scheduler })
    }
} else if #[cfg(feature = "sched_rms")] {
    pub fn new(runtime: usize, period: usize) -> SpinNoIrq<Self> {
        let gc_task: Arc<scheduler::RMSTask<TaskInner>> = 
            TaskInner::new(gc_entry, "gc".into(), axconfig::TASK_STACK_SIZE, runtime, period);
        let mut scheduler = Scheduler::new();
        scheduler.add_task(gc_task);
        SpinNoIrq::new(Self { scheduler })
    }
} else {
    // pub fn new() -> SpinNoIrq<Self> {
    //     let gc_task = TaskInner::new(gc_entry, "gc".into(), axconfig::TASK_STACK_SIZE);
    //     let mut scheduler = Scheduler::new();
    //     scheduler.add_task(gc_task);
    //     SpinNoIrq::new(Self { scheduler })
    // }
    pub fn new(hartid: usize) -> Arc<SpinNoIrq<Self>> {
        let gc_task = TaskInner::new(gc_entry, "gc".into(), axconfig::TASK_STACK_SIZE);
        let scheduler = Arc::new(SpinNoIrq::new(Scheduler::new()));
        REAL_RUN_QUEUE[hartid].init_by(scheduler.clone());
        let tmp = Arc::new(SpinNoIrq::new(Self { scheduler: scheduler.clone() })) ;
        let tmp_as_dyn = scheduler.clone() as Arc<SpinNoIrq<dyn BaseScheduler<SchedItem = Arc<AxTask>> + Send + 'static>>;
        RUN_MANAGER.lock().init(hartid, tmp_as_dyn.clone());
        RUN_MANAGER.lock().add_task(hartid, gc_task);
        tmp
    }
}
}

    pub fn add_task(&mut self, task: AxTaskRef) {
        debug!("task spawn: {}", task.id_name());
        assert!(task.is_ready());
        RUN_MANAGER.lock().add_task(get_current_cpu_id(), task);
    }

    #[cfg(feature = "irq")]
    pub fn scheduler_timer_tick(&mut self) {
        let curr = crate::current();
        if !curr.is_idle() && RUN_MANAGER.lock().task_tick(get_current_cpu_id(), curr.as_task_ref()) {
            #[cfg(feature = "preempt")]
            curr.set_preempt_pending(true);
        }
    }

    pub fn yield_current(&mut self) {
        let curr = crate::current();
        debug!("task yield: {}", curr.id_name());
        assert!(curr.is_running());
        self.resched_inner(false);
    }

    #[cfg(feature = "preempt")]
    pub fn resched(&mut self) {
        let curr = crate::current();
        assert!(curr.is_running());

        // When we get the mutable reference of the run queue, we must
        // have held the `SpinNoIrq` lock with both IRQs and preemption
        // disabled. So we need to set `current_disable_count` to 1 in
        // `can_preempt()` to obtain the preemption permission before
        //  locking the run queue.
        let can_preempt = curr.can_preempt(1);

        debug!(
            "current task is to be preempted: {}, allow={}",
            curr.id_name(),
            can_preempt
        );
        if can_preempt {
            self.resched_inner(true);
        } else {
            curr.set_preempt_pending(true);
        }
    }

    pub fn exit_current(&mut self, exit_code: i32) -> ! {
        let curr = crate::current();
        debug!("task exit: {}, exit_code={}", curr.id_name(), exit_code);
        assert!(curr.is_running());
        assert!(!curr.is_idle());
        if curr.is_init() {
            EXITED_TASKS.lock().clear();
            axhal::misc::terminate();
        } else {
            curr.set_state(TaskState::Exited);
            EXITED_TASKS.lock().push_back(curr.clone());
            WAIT_FOR_EXIT.notify_one_locked(false, self);
            self.resched_inner(false);
        }
        unreachable!("task exited!");
    }

    pub fn block_current<F>(&mut self, wait_queue_push: F)
    where
        F: FnOnce(AxTaskRef),
    {
        let curr = crate::current();
        debug!("task block: {}", curr.id_name());
        assert!(curr.is_running());
        assert!(!curr.is_idle());

        // we must not block current task with preemption disabled.
        #[cfg(feature = "preempt")]
        assert!(curr.can_preempt(1));

        curr.set_state(TaskState::Blocked);
        wait_queue_push(curr.clone());
        self.resched_inner(false);
    }

    pub fn unblock_task(&mut self, task: AxTaskRef, resched: bool) {
        debug!("task unblock: {}", task.id_name());
        if task.is_blocked() {
            task.set_state(TaskState::Ready);
            RUN_MANAGER.lock().add_task(get_current_cpu_id(), task);
            if resched {
                #[cfg(feature = "preempt")]
                crate::current().set_preempt_pending(true);
            }
        }
    }

    #[cfg(feature = "irq")]
    pub fn sleep_until(&mut self, deadline: axhal::time::TimeValue) {
        let curr = crate::current();
        debug!("task sleep: {}, deadline={:?}", curr.id_name(), deadline);
        assert!(curr.is_running());
        assert!(!curr.is_idle());

        let now = axhal::time::current_time();
        if now < deadline {
            crate::timers::set_alarm_wakeup(deadline, curr.clone());
            curr.set_state(TaskState::Blocked);
            self.resched_inner(false);
        }
    }
}

impl AxRunQueue {
    /// Common reschedule subroutine. If `preempt`, keep current task's time
    /// slice, otherwise reset it.
    fn resched_inner(&mut self, preempt: bool) {
        let prev = crate::current();
        if prev.is_running() {
            prev.set_state(TaskState::Ready);
            if !prev.is_idle() {
                RUN_MANAGER.lock().put_prev_task(get_current_cpu_id(), prev.clone(), preempt);
            }
        }
        let next = RUN_MANAGER.lock().pick_next_task(get_current_cpu_id()).unwrap_or_else(|| unsafe {
            // Safety: IRQs must be disabled at this time.
            IDLE_TASK.current_ref_raw().get_unchecked().clone()
        });
        self.switch_to(prev, next);
    }

    fn switch_to(&mut self, prev_task: CurrentTask, next_task: AxTaskRef) {
        trace!(
            "context switch: {} -> {}",
            prev_task.id_name(),
            next_task.id_name()
        );
        trace!(
            "Arc: prev {}", Arc::strong_count(prev_task.as_task_ref()),
        );
        trace!(
            "Arc: next {}", Arc::strong_count(&next_task),
        );
        #[cfg(feature = "preempt")]
        next_task.set_preempt_pending(false);
        next_task.set_state(TaskState::Running);
        if prev_task.ptr_eq(&next_task) {
            return;
        }

        unsafe {
            let prev_ctx_ptr = prev_task.ctx_mut_ptr();
            let next_ctx_ptr = next_task.ctx_mut_ptr();

            // The strong reference count of `prev_task` will be decremented by 1,
            // but won't be dropped until `gc_entry()` is called.
            assert!(Arc::strong_count(prev_task.as_task_ref()) > 1);
            assert!(Arc::strong_count(&next_task) >= 1);

            CurrentTask::set_current(prev_task, next_task);
            (*prev_ctx_ptr).switch_to(&*next_ctx_ptr);
            
        }
    }
}

fn gc_entry() {
    loop {
        // Drop all exited tasks and recycle resources.
        while !EXITED_TASKS.lock().is_empty() {
            // Do not do the slow drops in the critical section.
            let task = EXITED_TASKS.lock().pop_front();
            if let Some(task) = task {
                // wait for other threads to release the reference.
                while Arc::strong_count(&task) > 1 {
                    core::hint::spin_loop();
                }
                drop(task);
            }
        }
        WAIT_FOR_EXIT.wait();
    }
}

cfg_if::cfg_if! {
if #[cfg(feature = "sched_cfs")] {
    pub(crate) fn init() {
        const IDLE_TASK_STACK_SIZE: usize = 4096;
        let idle_task = TaskInner::new(|| crate::run_idle(), "idle".into(), IDLE_TASK_STACK_SIZE, 0);
        IDLE_TASK.with_current(|i| i.init_by(idle_task.clone()));
    
        let main_task = TaskInner::new_init("main".into());
        main_task.set_state(TaskState::Running);
    
        RUN_QUEUE.init_by(AxRunQueue::new(0));
        unsafe { CurrentTask::init_current(main_task) }
    }
} else if #[cfg(feature = "sched_rms")] {
    pub(crate) fn init() {
        const IDLE_TASK_STACK_SIZE: usize = 4096;
        let idle_task = TaskInner::new(|| crate::run_idle(), "idle".into(), IDLE_TASK_STACK_SIZE, 0, 1);
        IDLE_TASK.with_current(|i| i.init_by(idle_task.clone()));
    
        let main_task = TaskInner::new_init("main".into());
        main_task.set_state(TaskState::Running);
    
        RUN_QUEUE.init_by(AxRunQueue::new(0, 1));
        unsafe { CurrentTask::init_current(main_task) }
    }
} else {
    pub(crate) fn init() {
        RUN_MANAGER.init_by(SpinNoIrq::new(Manager::new()));
        for i in 0..axconfig::SMP {
            RUN_QUEUE[i].init_by(AxRunQueue::new(i));
        }
        const IDLE_TASK_STACK_SIZE: usize = 4096;
        let idle_task = TaskInner::new(|| crate::run_idle(), "idle".into(), IDLE_TASK_STACK_SIZE);
        IDLE_TASK.with_current(|i| i.init_by(idle_task.clone()));     
        let main_task = TaskInner::new_init("main".into());
        main_task.set_state(TaskState::Running);
    
        unsafe { CurrentTask::init_current(main_task) }
    }
}
}

pub(crate) fn init_secondary() {
    let idle_task = TaskInner::new_init("idle".into());
    idle_task.set_state(TaskState::Running);
    IDLE_TASK.with_current(|i| i.init_by(idle_task.clone()));
    unsafe { CurrentTask::init_current(idle_task) }
}