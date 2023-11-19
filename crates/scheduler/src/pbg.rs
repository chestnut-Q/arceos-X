use core::{sync::atomic::{AtomicIsize, Ordering}, ops::Deref};
use alloc::{vec::Vec, sync::Arc};

use crate::BaseScheduler;

pub struct PBGTask<T> {
    inner: T,
    id: AtomicIsize,
}

impl<T> PBGTask<T> {
    pub const fn new(inner: T) -> Self {
        Self {
            inner,
            id: AtomicIsize::new(0 as isize),
        }
    }

    pub fn set_id(&self, id: isize) {
        self.id.store(id, Ordering::Release);
    }

    pub fn get_id(&self) -> isize {
        self.id.load(Ordering::Acquire)
    }
}

impl<T> Deref for PBGTask<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct PBGScheduler<T> {
    ready_queue: Vec<Arc<PBGTask<T>>>,
    id: AtomicIsize,
}

impl<T> PBGScheduler<T> {
    pub const fn new() -> Self {
        Self {
            ready_queue: Vec::new(),
            id: AtomicIsize::new(0 as isize),
        }
    }

    /// get the name of the scheduler
    pub fn scheduler_name(&self) -> &'static str {
        "Program Behavior Guided"
    }
}

impl<T> BaseScheduler for PBGScheduler<T> {
    type SchedItem = Arc<PBGTask<T>>;

    fn init(&mut self) {
        self.ready_queue.clear();
    }

    fn add_task(&mut self, task: Self::SchedItem) {
        (*task).set_id(self.id.fetch_add(1, Ordering::Release));
        self.ready_queue.push(task);
    }

    fn remove_task(&mut self, task: &Self::SchedItem) -> Option<Self::SchedItem> {
        let id: isize = task.get_id();
        if let Some(index) = self.ready_queue.iter().position(|x| x.get_id() == id) {
            Some(self.ready_queue.remove(index))
        } else {
            None
        }
    }

    fn pick_next_task(&mut self) -> Option<Self::SchedItem> {
        if let Some(task) = self.ready_queue.pop() {
            Some(task)
        } else {
            None
        }
    }

    fn put_prev_task(&mut self, prev: Self::SchedItem, preempt: bool) {
        self.ready_queue.push(prev);
    }

    fn task_tick(&mut self, _current: &Self::SchedItem) -> bool {
        false
    }

    fn set_priority(&mut self, _task: &Self::SchedItem, _prio: isize) -> bool {
        false
    }    
}