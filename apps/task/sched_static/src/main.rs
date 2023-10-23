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

const NUM_DATA_RUNTIME_1: usize = 1;
const NUM_DATA_RUNTIME_2: usize = 2;
const NUM_DATA_RUNTIME_3: usize = 1;
const NUM_DATA_PERIOD_1: usize = 3;
const NUM_DATA_PERIOD_2: usize = 5;
const NUM_DATA_PERIOD_3: usize = 6;
const NUM_RUN_TIMES    : usize = 1000; 
const PAYLOAD_KIND     : usize = 3;


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

fn load(runtime: usize, sleeptime: usize) -> u64 {
    // 一个高耗时负载
    let mut sum : u64 = runtime as u64;
    for k in 0..runtime {
        println!("runtime = {}, sleeptime = {}", runtime, sleeptime);
        for i in 0..16000000 { // 每 runtime ~50ms
            sum = sum + ((i * i) ^ (i + runtime as u64)) / (i + 1);
        }
        if k + 1 != runtime {
            thread::yield_now();
        }
    }
    thread::sleep(Duration::from_millis(50 * sleeptime as u64));
    sum
}

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    let timeout = api::ax_wait_timeout(&MAIN_WQ, Duration::from_millis(500));
    assert!(timeout);

    for ii in 0..PAYLOAD_KIND {
        let i = PAYLOAD_KIND - 1 - ii; 
        let datalen: usize;
        let sleeplen: usize;
        if i == 0 {
            datalen = NUM_DATA_RUNTIME_1;
            sleeplen = NUM_DATA_PERIOD_1 - NUM_DATA_RUNTIME_1;
        } else if i == 1 {
            datalen = NUM_DATA_RUNTIME_2;
            sleeplen = NUM_DATA_PERIOD_2 - NUM_DATA_RUNTIME_2;
        } else if i == 2 {
            datalen = NUM_DATA_RUNTIME_3;
            sleeplen = NUM_DATA_PERIOD_3 - NUM_DATA_RUNTIME_3;
        } else {
            datalen = 0;
            sleeplen = 0;
        }
        thread::spawn(move || {
            let start_time = std::time::Instant::now();
            let left = 0;
            let right = NUM_RUN_TIMES;
            println!(
                "part {}: {:?} [{}, {})",
                i,
                thread::current().id(),
                left,
                right
            );
            let mut tmp: u64 = 0;
            for i in left..right {
                tmp += load(datalen, sleeplen);
            }
            RESULTS.lock()[i] = tmp;
            LEAVE_TIME.lock()[i] = start_time.elapsed().as_millis() as u64;

            //barrier();

            println!("part {}: {:?} finished", i, thread::current().id());
            let n = FINISHED_TASKS.fetch_add(1, Ordering::Relaxed);
            if i == PAYLOAD_KIND - 1 { // 注意这里只要高耗时进程结束就退出
                api::ax_wait_queue_wake(&MAIN_WQ, 1);
            }
        // }, datalen, datalen + sleeplen);
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
