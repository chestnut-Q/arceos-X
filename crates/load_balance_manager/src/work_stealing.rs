// 工作窃取 (work-stealing) 算法
// 要求：所有的调度器必须提供方法 is_empty(队列是否已空) 和 pick_last_task(从队列后方获得任务)
// 简单起见，可以不实现 pick_last_task，这样不是标准的工作窃取算法，因为可以从头部拿东西，但这里实现了进程锁理论上不会有太多问题。
// 目前的实现也不需要提供 is_empty，因为可以直接在 Manager 记录这个值。这样 Scheduler 啥也不用动！

extern crate alloc;

use alloc::sync::Arc;
use core::ops::Deref;
use crate::BaseManager;
use scheduler::BaseScheduler;
use alloc::vec::Vec;
use spinlock::SpinNoIrq; // TODO: 不确定！！！
use log::info;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
//use std::marker::PhantomData;

pub struct WorkStealingManager<Task, const SMP: usize> {
    // 之后可以支持每个调度器是不同的。目前还不行。
    scheduler_collection: Vec<Option<Arc<SpinNoIrq<dyn BaseScheduler<SchedItem = Arc<Task>> + Send + 'static>>>>,
    // 记录每个调度器的任务个数
    task_counter: Vec<AtomicUsize>,
    // 记录每个调度器存着的任务个数。注意有些任务已被调度
    task_current_counter: Vec<AtomicUsize>,
}

impl<Task, const SMP: usize> WorkStealingManager<Task, SMP> {
    pub fn new() -> Self {
        let mut tmp_collection: Vec<Option<Arc<SpinNoIrq<dyn BaseScheduler<SchedItem = Arc<Task>> + Send + 'static>>>> = Vec::new();
        let mut task_counter: Vec<AtomicUsize> = Vec::new();
        let mut task_current_counter: Vec<AtomicUsize> = Vec::new();
        for _i in 0..SMP {
            tmp_collection.push(None);
            task_counter.push(AtomicUsize::new(0));
            task_current_counter.push(AtomicUsize::new(0));
        }
        Self {
            scheduler_collection: tmp_collection,
            task_counter,
            task_current_counter,
        }
    }
}

impl<Task, const SMP: usize> BaseManager for WorkStealingManager<Task, SMP> {
    type SchedItem = Arc<Task>;
    fn init(&mut self, cpu_id: usize, queue_ref: Arc<SpinNoIrq<dyn BaseScheduler<SchedItem = Self::SchedItem> + Send + 'static>>) {
        self.scheduler_collection[cpu_id] = Some(queue_ref.clone());
        queue_ref.lock().init();
    }

    fn add_task(&mut self, cpu_id: usize, task: Self::SchedItem) {
        self.task_counter[cpu_id].fetch_add(1, Ordering::Release);
        self.task_current_counter[cpu_id].fetch_add(1, Ordering::Release);
        self.scheduler_collection[cpu_id].as_ref().unwrap().lock().add_task(task);
    }

    fn remove_task(&mut self, cpu_id: usize, task: &Self::SchedItem) -> Option<Self::SchedItem> {
        self.task_current_counter[cpu_id].fetch_sub(1, Ordering::Release);
        if self.task_counter[cpu_id].fetch_sub(1, Ordering::Release) <= 1 {
            for i in 0..SMP {
                if i != cpu_id && self.task_current_counter[i].load(Ordering::Acquire) >= 1 {
                    let victim_task = self.pick_next_task(i).unwrap();
                    self.add_task(cpu_id, victim_task);
                    self.task_counter[i].fetch_sub(1, Ordering::Release);
                    self.task_current_counter[i].fetch_sub(1, Ordering::Release);
                    self.task_counter[cpu_id].fetch_add(1, Ordering::Release);
                    self.task_current_counter[i].fetch_add(1, Ordering::Release);
                }
            }
        }
        self.scheduler_collection[cpu_id].as_ref().unwrap().lock().remove_task(task)
    }

    fn pick_next_task(&mut self, cpu_id: usize) -> Option<Self::SchedItem> {
        self.task_current_counter[cpu_id].fetch_sub(1, Ordering::Release);
        self.scheduler_collection[cpu_id].as_ref().unwrap().lock().pick_next_task()
    }

    fn put_prev_task(&mut self, cpu_id: usize, prev: Self::SchedItem, _preempt: bool) {
        self.task_current_counter[cpu_id].fetch_add(1, Ordering::Release);
        self.scheduler_collection[cpu_id].as_ref().unwrap().lock().put_prev_task(prev, _preempt);
    }

    fn task_tick(&mut self, cpu_id: usize, _current: &Self::SchedItem) -> bool {
        self.scheduler_collection[cpu_id].as_ref().unwrap().lock().task_tick(_current)
    }
}
