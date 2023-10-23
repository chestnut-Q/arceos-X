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

const NUM_DATA: usize = 20000; // 充分多
const NUM_TASKS: usize = 4;

static FINISHED_TASKS: AtomicUsize = AtomicUsize::new(0);

static MAIN_WQ: AxWaitQueueHandle = AxWaitQueueHandle::new();
static RESULTS: Mutex<[u64; NUM_TASKS]> = Mutex::new([0; NUM_TASKS]); // TODO: task join
static LEAVE_TIME: Mutex<[u64; NUM_TASKS]> = Mutex::new([0; NUM_TASKS]);
static CALCS: Mutex<[u64; NUM_TASKS]> = Mutex::new([0; NUM_TASKS]);

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
    // 一个高耗时负载，运行 1000+n 次
    let mut sum : u64 = *n;
    for i in 0..(50000000 + (*n / (NUM_DATA / NUM_TASKS) as u64 * 50000000)) {
        sum = sum + ((i ^ (i + *n)) >> 10);
    }
    thread::yield_now();
    sum
}

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    // 设置每个的运行次数相同，每个 f(i) 次循环后 yield 一次，i 是测例编号，f(i) 单增
    // 期望：对于公平算法，如果是自动设置公平性，退出时间应该基本一致。
    let vec = Arc::new(
        (0..NUM_DATA)
            .map(|idx| idx as u64)
            .collect::<Vec<_>>(),
    );
    //let expect: u64 = vec.iter().map(load).sum();

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

            for j in left..right {
                RESULTS.lock()[i] += load(&vec[j]);
                CALCS.lock()[i] += 1;
            }

            barrier();

            println!("part {}: {:?} finished", i, thread::current().id());
            let n = FINISHED_TASKS.fetch_add(1, Ordering::Relaxed);
            if n == NUM_TASKS - 1 {
                api::ax_wait_queue_wake(&MAIN_WQ, 1);
            }
        });
    }

    let timeout = api::ax_wait_timeout(&MAIN_WQ, Duration::from_millis(5000));
    println!("main task woken up! timeout={}", timeout);

    //let actual = RESULTS.lock().iter().sum();
    let binding2 = CALCS.lock();
    for i in 0..NUM_TASKS {
        println!("id {}, calc times = {}", i, binding2[i]);
    }
    //assert_eq!(expect, actual);

    println!("Parallel summation tests run OK!");
}
