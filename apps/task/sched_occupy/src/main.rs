#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[macro_use]
#[cfg(feature = "axstd")]
extern crate axstd as std;

use std::{sync::Arc, vec::Vec};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use std::os::arceos::api::task::{self as api, AxWaitQueueHandle};
use std::sync::Mutex;
use std::thread;

const NUM_DATA_SHORT_1: usize = 0;
const NUM_DATA_SHORT_2: usize = 0;
const NUM_DATA_SHORT_3: usize = 2000000;
const NUM_DATA_SHORT_4: usize = 2000000;
const NUM_DATA_LONG_1 : usize = 80;
const NUM_DATA_SHORT_LOAD_1: usize = 0;
const NUM_DATA_SHORT_LOAD_2: usize = 0;
const NUM_DATA_SHORT_LOAD_3: usize = 100;
const NUM_DATA_SHORT_LOAD_4: usize = 100;
const NUM_DATA_LONG_LOAD_1: usize = 10000000;
const PAYLOAD_KIND         : usize = 5;


static FINISHED_TASKS: AtomicUsize = AtomicUsize::new(0);

static MAIN_WQ: AxWaitQueueHandle = AxWaitQueueHandle::new();
static RESULTS: Mutex<[u64; PAYLOAD_KIND]> = Mutex::new([0; PAYLOAD_KIND]); // TODO: task join
static LEAVE_TIME: Mutex<[u64; PAYLOAD_KIND]> = Mutex::new([0; PAYLOAD_KIND]);

#[cfg(feature = "axstd")]
fn barrier() {
    static BARRIER_WQ: AxWaitQueueHandle = AxWaitQueueHandle::new();
    static BARRIER_COUNT: AtomicUsize = AtomicUsize::new(0);

    BARRIER_COUNT.fetch_add(1, Ordering::Relaxed);
    api::ax_wait_queue_wait(
        &BARRIER_WQ,
        || BARRIER_COUNT.load(Ordering::Relaxed) == PAYLOAD_KIND,
        None,
    );
    api::ax_wait_queue_wake(&BARRIER_WQ, u32::MAX); // wakeup all
}

fn load(n: &u64) -> u64 {
    // 一个高耗时负载，运行 1000+n 次
    let mut sum : u64 = *n;
    for i in 0..*n {
        sum = sum + ((i * i) ^ (i + *n)) / (i + 1);
    }
    thread::yield_now();
    sum
}

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    // 混合设置一些短测例和几个长测例，观察实时性
    let vec_short1 = Arc::new(
        (0..NUM_DATA_SHORT_1)
            .map(|_| NUM_DATA_SHORT_LOAD_1 as u64)
            .collect::<Vec<_>>(),
    );
    let vec_short2 = Arc::new(
        (0..NUM_DATA_SHORT_2)
            .map(|_| NUM_DATA_SHORT_LOAD_2 as u64)
            .collect::<Vec<_>>(),
    );
    let vec_short3 = Arc::new(
        (0..NUM_DATA_SHORT_3)
            .map(|_| NUM_DATA_SHORT_LOAD_3 as u64)
            .collect::<Vec<_>>(),
    );
    let vec_short4 = Arc::new(
        (0..NUM_DATA_SHORT_4)
            .map(|_| NUM_DATA_SHORT_LOAD_4 as u64)
            .collect::<Vec<_>>(),
    );
    let vec_long1 = Arc::new(
        (0..NUM_DATA_LONG_1)
            .map(|_| NUM_DATA_LONG_LOAD_1 as u64)
            .collect::<Vec<_>>(),
    );
    /*let expect: u64 = vec_short1.iter().map(load).sum::<u64>()
                    + vec_short2.iter().map(load).sum::<u64>()
                    + vec_short3.iter().map(load).sum::<u64>()
                    + vec_short4.iter().map(load).sum::<u64>()
                    + vec_long1.iter().map(load).sum::<u64>();*/

    let timeout = api::ax_wait_timeout(&MAIN_WQ, Duration::from_millis(500));
    assert!(timeout);

    for ii in 0..PAYLOAD_KIND {
        let i = PAYLOAD_KIND - 1 - ii; 
        let vec: Arc<Vec<u64>>;
        let datalen: usize;
        if i == 0 {
            vec = vec_short1.clone();
            datalen = NUM_DATA_SHORT_1;
        } else if i == 1 {
            vec = vec_short2.clone();
            datalen = NUM_DATA_SHORT_2;
        } else if i == 2 {
            vec = vec_short3.clone();
            datalen = NUM_DATA_SHORT_3;
        } else if i == 3 {
            vec = vec_short4.clone();
            datalen = NUM_DATA_SHORT_4;
        } else if i == 4 {
            vec = vec_long1.clone();
            datalen = NUM_DATA_LONG_1;
        } else {
            vec = Arc::new(Vec::new());
            datalen = 0;
        }
        thread::spawn(move || {
            let start_time = std::time::Instant::now();
            let left = 0;
            let right = datalen;
            println!(
                "part {}: {:?} [{}, {})",
                i,
                thread::current().id(),
                left,
                right
            );

            RESULTS.lock()[i] = vec[left..right].iter().map(load).sum();
            LEAVE_TIME.lock()[i] = start_time.elapsed().as_millis() as u64;

            //barrier();

            println!("part {}: {:?} finished", i, thread::current().id());
            let n = FINISHED_TASKS.fetch_add(1, Ordering::Relaxed);
            if i == PAYLOAD_KIND - 1 { // 注意这里只要高耗时进程结束就退出
                api::ax_wait_queue_wake(&MAIN_WQ, 1);
            }
        });
    }

    let timeout = api::ax_wait_timeout(&MAIN_WQ, Duration::from_millis(20000));
    println!("main task woken up! timeout={}", timeout);

    //let actual = RESULTS.lock().iter().sum();
    let binding = LEAVE_TIME.lock();
    let long_task_leave_time = binding[PAYLOAD_KIND - 1];
    println!("long task leave time = {}ms", long_task_leave_time);
    drop(binding);
    //println!("sum = {}", actual);
    //assert_eq!(expect, actual);

    println!("Parallel summation tests run OK!");
}
