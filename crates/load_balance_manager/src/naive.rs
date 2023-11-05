// 啥调度也没有，就是一个封装

extern crate alloc;

use alloc::sync::Arc;
use core::ops::Deref;
use crate::BaseManager;
use scheduler::BaseScheduler;
use alloc::vec::Vec;
use spinlock::SpinNoIrq; // TODO: 不确定！！！
use log::info;
//use std::marker::PhantomData;

pub struct NaiveManager<Task, const SMP: usize> {
    scheduler_collection: Vec<Option<Arc<SpinNoIrq<dyn BaseScheduler<SchedItem = Arc<Task>> + Send + 'static>>>>,
}

impl<Task, const SMP: usize> NaiveManager<Task, SMP> {
    pub fn new() -> Self {
        let mut tmp_collection: Vec<Option<Arc<SpinNoIrq<dyn BaseScheduler<SchedItem = Arc<Task>> + Send + 'static>>>> = Vec::new();
        for _i in 0..SMP {
            tmp_collection.push(None);
        }
        Self {
            scheduler_collection: tmp_collection,
        }
    }
}

impl<Task, const SMP: usize> BaseManager for NaiveManager<Task, SMP> {
    type SchedItem = Arc<Task>;
    fn init(&mut self, cpu_id: usize, queue_ref: Arc<SpinNoIrq<dyn BaseScheduler<SchedItem = Self::SchedItem> + Send + 'static>>) {
        self.scheduler_collection[cpu_id] = Some(queue_ref.clone());
        queue_ref.lock().init();
    }

    fn add_task(&mut self, cpu_id: usize, task: Self::SchedItem) {
        self.scheduler_collection[cpu_id].as_ref().unwrap().lock().add_task(task);
    }

    fn remove_task(&mut self, cpu_id: usize, task: &Self::SchedItem) -> Option<Self::SchedItem> {
        self.scheduler_collection[cpu_id].as_ref().unwrap().lock().remove_task(task)
    }

    fn pick_next_task(&mut self, cpu_id: usize) -> Option<Self::SchedItem> {
        self.scheduler_collection[cpu_id].as_ref().unwrap().lock().pick_next_task()
    }

    fn put_prev_task(&mut self, cpu_id: usize, prev: Self::SchedItem, _preempt: bool) {
        self.scheduler_collection[cpu_id].as_ref().unwrap().lock().put_prev_task(prev, _preempt);
    }

    fn task_tick(&mut self, cpu_id: usize, _current: &Self::SchedItem) -> bool {
        self.scheduler_collection[cpu_id].as_ref().unwrap().lock().task_tick(_current);
        false
    }
}
