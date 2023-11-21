use core::{sync::atomic::{AtomicIsize, Ordering}, ops::Deref};
use alloc::{vec::Vec, sync::Arc, collections::VecDeque};

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

/// PBGTaskInfo
pub struct PBGTaskInfo {
    // id
    pub id: isize,
    // exeuted_time
    pub exeuted_time: u32,
    // exeuted_count
    pub exeuted_count: u32,
}

pub struct PBGScheduler<T> {
    ready_queue: Vec<Arc<PBGTask<T>>>,
    exeuted_queue: Vec<Arc<PBGTask<T>>>,
    exeuted_id_queue: Vec<isize>,
    exeuted_count_queue: Vec<isize>,
    id: AtomicIsize,
    is_using_pbg: bool,
    is_profiling: bool,
    specified_id_list: VecDeque<isize>,
    specified_id_list_backup: VecDeque<isize>,
}

impl<T> PBGScheduler<T> {
    pub const fn new() -> Self {
        Self {
            ready_queue: Vec::new(),
            exeuted_queue: Vec::new(),
            exeuted_id_queue: Vec::new(),
            exeuted_count_queue: Vec::new(),
            id: AtomicIsize::new(0 as isize),
            is_using_pbg: false,
            is_profiling: false,
            specified_id_list: VecDeque::new(),
            specified_id_list_backup: VecDeque::new(),
        }
    }

    /// get the name of the scheduler
    pub fn scheduler_name() -> &'static str {
        "Program Behavior Guided"
    }

    pub fn profile_task_info(&mut self) {

    }

    pub fn open_profile(&mut self, file_name: &str) -> bool {
        self.is_profiling = true;
        false
    }

    pub fn open_pbg(&mut self, file_name: &str, task_infos: &mut Vec<PBGTaskInfo>) {
        if file_name == "profile" {
            self.is_using_pbg = false;
        } else {
            self.is_using_pbg = true;
            self.set_task_info(task_infos);
        }
    }

    pub fn close_profile(&mut self) {
        self.is_profiling = false;
    }

    pub fn set_task_info(&mut self, task_infos: &mut Vec<PBGTaskInfo>) {
        for task_info in task_infos.iter_mut() {
            task_info.id = self.id.fetch_add(1, Ordering::Release);
            self.specified_id_list.push_back(task_info.id);
        }
        self.specified_id_list_backup = self.specified_id_list.clone();
        task_infos.sort_by_key(|task: &PBGTaskInfo| task.exeuted_time);
        for task_info in task_infos.iter() {
            self.exeuted_id_queue.push(task_info.id);
            self.exeuted_count_queue.push(task_info.exeuted_count as isize);
        }
    }
}

impl<T> BaseScheduler for PBGScheduler<T> {
    type SchedItem = Arc<PBGTask<T>>;

    fn init(&mut self) {
        self.ready_queue.clear();
    }

    fn add_task(&mut self, task: Self::SchedItem) {
        if self.is_using_pbg && !self.specified_id_list.is_empty() {
            let id = self.specified_id_list.pop_front().unwrap();
            (*task).set_id(id);
            self.exeuted_queue.push(task);
        } else {
        (*task).set_id(self.id.fetch_add(1, Ordering::Release));
        (*task).set_id(self.id.fetch_add(1, Ordering::Release));
            (*task).set_id(self.id.fetch_add(1, Ordering::Release));
            self.ready_queue.push(task);
        }
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
        if self.is_using_pbg && !self.exeuted_id_queue.is_empty(){
            let mut id = -1;
            for i in 0..self.exeuted_id_queue.len() {
                if self.exeuted_count_queue[i] == 0 {
                    continue;
                } else {
                    self.exeuted_count_queue[i] = self.exeuted_count_queue[i] - 1;
                    id = self.exeuted_id_queue[i];
                    break;
                }
            }
            if id != -1 {
                let found_task: Option<&Arc<PBGTask<T>>> = self.exeuted_queue.iter().find(|task| task.get_id() == id);
                found_task.cloned()
            } else {
                if let Some(task) = self.ready_queue.pop() {
                    Some(task)
                } else {
                    None
                }
            }
        } else {
            if let Some(task) = self.ready_queue.pop() {
                Some(task)
            } else {
                None
            }
        }
    }

    fn put_prev_task(&mut self, prev: Self::SchedItem, preempt: bool) {
        if !self.exeuted_id_queue.is_empty() {
            let id = (prev.get_id() - self.exeuted_id_queue[0]) as usize;
            if id > self.exeuted_count_queue.len() || self.exeuted_count_queue[id] == 0 {
                self.ready_queue.push(prev);
            }
        } else {
            self.ready_queue.push(prev);
        }
        
    }

    fn task_tick(&mut self, _current: &Self::SchedItem) -> bool {
        false
    }

    fn set_priority(&mut self, _task: &Self::SchedItem, _prio: isize) -> bool {
        false
    }    
}