#![no_std]
#![no_main]

#[macro_use]
extern crate libax;
extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::time::Duration;
use libax::sync::{Mutex, WaitQueue};
use libax::{rand, task};
use libax::task::{sleep, yield_now};

const NUM_TASKS: usize = 20;

static FINISHED_TASKS: AtomicUsize = AtomicUsize::new(0);

static MAIN_WQ: WaitQueue = WaitQueue::new();
static RESULTS: Mutex<[u64; NUM_TASKS]> = Mutex::new([0; NUM_TASKS]); // TODO: task join
static LEAVE_TIME: Mutex<[u64; NUM_TASKS]> = Mutex::new([0; NUM_TASKS]);
static CALCS: Mutex<[u64; NUM_TASKS]> = Mutex::new([0; NUM_TASKS]);

fn barrier() {
    static BARRIER_WQ: WaitQueue = WaitQueue::new();
    static BARRIER_COUNT: AtomicUsize = AtomicUsize::new(0);
    BARRIER_COUNT.fetch_add(1, Ordering::Relaxed);
    BARRIER_WQ.wait_until(|| BARRIER_COUNT.load(Ordering::Relaxed) == NUM_TASKS);
    BARRIER_WQ.notify_all(true);
}

fn load(n: &u64) -> u64 {
    let mut sum : u64 = *n;
    for i in 0..(1 << 25) {
        sum = sum + ((i ^ (i + *n)) >> 10);
    }
    yield_now();
    sum
}

#[no_mangle]
fn main() {
    //let expect: u64 = vec.iter().map(load).sum();

    let timeout = MAIN_WQ.wait_timeout(Duration::from_millis(500));
    assert!(timeout);

    for i in 0..NUM_TASKS {
        task::spawn(move || {
            let start_time = libax::time::Instant::now();
            let left = 0;
            let right = ((i % 4) * 4 + 1) as u64;
            println!(
                "part {}: {:?} [{}, {})",
                i,
                task::current().id(),
                left,
                right
            );

            for j in left..right {
                let tmp = rand::rand_u32() as u64;
                RESULTS.lock()[i] += load(&tmp);
            }
            LEAVE_TIME.lock()[i] = start_time.elapsed().as_millis() as u64;

            barrier();

            println!("part {}: {:?} finished", i, task::current().id());
            let n = FINISHED_TASKS.fetch_add(1, Ordering::Relaxed);
            if n == NUM_TASKS - 1 {
                MAIN_WQ.notify_one(true);
            }
        });
    }

    let timeout = MAIN_WQ.wait_timeout(Duration::from_millis(12000));
    let binding = LEAVE_TIME.lock();
    for i in 0..NUM_TASKS {
        println!("leave time id {} = {}ms", i, binding[i]);
    }
    drop(binding);
    println!("main task woken up! timeout={}", timeout);
    let binding = LEAVE_TIME.lock();
    let max_leave_time = binding.iter().max();
    println!("maximum leave time = {}ms", max_leave_time.unwrap());
    println!("Parallel summation tests run OK!");
}
