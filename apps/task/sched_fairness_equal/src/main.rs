#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[macro_use]
#[cfg(feature = "axstd")]
extern crate axstd as std;

use std::os::arceos::api::task::{self as api, AxWaitQueueHandle};

use std::sync::atomic::{AtomicUsize, Ordering};
use std::{sync::Arc, vec::Vec};
use std::time::Duration;
use std::sync::Mutex;
use std::thread;


const NUM_DATA: usize = 100;
const NUM_TASKS: usize = 4;
const INCREMENT_SCALE: u64 = 0;

static FINISHED_TASKS: AtomicUsize = AtomicUsize::new(0);
static MAIN_WQ: AxWaitQueueHandle = AxWaitQueueHandle::new();

static RESULTS: Mutex<[u64; NUM_TASKS]> = Mutex::new([0; NUM_TASKS]); // TODO: task join
static LEAVE_TIME: Mutex<[u64; NUM_TASKS]> = Mutex::new([0; NUM_TASKS]);

#[cfg(feature = "axstd")]
fn barrier() {
    static BARRIER_WQ: AxWaitQueueHandle = AxWaitQueueHandle::new();
    static BARRIER_COUNT: AtomicUsize = AtomicUsize::new(0);

    BARRIER_COUNT.fetch_add(1, Ordering::Relaxed);
    api::ax_wait_queue_wait(
        &BARRIER_WQ,
        || BARRIER_COUNT.load(Ordering::Relaxed) == NUM_TASKS,
        None,
    );
    api::ax_wait_queue_wake(&BARRIER_WQ, u32::MAX); // wakeup all
}

fn load(n: &u64) -> u64 {
    let mut sum : u64 = *n;
    for i in 0..5000000 + INCREMENT_SCALE * (*n) {
        sum = sum + ((i * i) ^ (i + *n)) / (i + 1);
    }
    thread::yield_now();
    sum
}

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    // 设置每个的运行次数**几乎**相同，每个 1000000 + INCREMENT_SCALE * i 次循环后 yield 一次，i 是测例编号。
    // 期望：对于公平算法，如果是自动设置公平性，退出时间应该基本一致。
    let vec = Arc::new(
        (0..NUM_DATA)
            .map(|idx| idx as u64)
            .collect::<Vec<_>>(),
    );
    let expect: u64 = vec.iter().map(load).sum();

    let timeout = api::ax_wait_timeout(&MAIN_WQ, Duration::from_millis(500));
    assert!(timeout);

    for i in 0..NUM_TASKS {
        let vec = vec.clone();
        thread::spawn(move || {
            let start_time = std::time::Instant::now();
            let left = i * (NUM_DATA / NUM_TASKS);
            let right = (left + (NUM_DATA / NUM_TASKS)).min(NUM_DATA);
            println!(
                "part {}: {:?} [{}, {})",
                i,
                thread::current().id(),
                left,
                right
            );

            RESULTS.lock()[i] = vec[left..right].iter().map(load).sum();
            LEAVE_TIME.lock()[i] = start_time.elapsed().as_millis() as u64;

            barrier();

            println!("part {}: {:?} finished", i, thread::current().id());
            let n = FINISHED_TASKS.fetch_add(1, Ordering::Relaxed);
            if n == NUM_TASKS - 1 {
                api::ax_wait_queue_wake(&MAIN_WQ, 1);
            }
        }, 0);
    }
    let timeout = api::ax_wait_timeout(&MAIN_WQ, Duration::from_millis(20000));
    println!("main task woken up! timeout={}", timeout);

    let actual = RESULTS.lock().iter().sum();
    let binding = LEAVE_TIME.lock();
    for i in 0..NUM_TASKS {
        println!("leave time id {} = {}ms", i, binding[i]);
    }
    drop(binding);
    let binding = LEAVE_TIME.lock();
    let max_leave_time = binding.iter().max();
    println!("maximum leave time = {}ms", max_leave_time.unwrap());
    drop(binding);
    println!("sum = {}", actual);
    let binding = LEAVE_TIME.lock();
    let min_leave_time = binding.iter().min();
    println!("minimum leave time = {}ms", min_leave_time.unwrap());
    assert_eq!(expect, actual);

    println!("Parallel summation tests run OK!");
}
