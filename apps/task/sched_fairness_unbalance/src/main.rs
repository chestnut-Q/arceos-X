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
use rand::{rngs::SmallRng, RngCore, SeedableRng};

const NUM_TASKS: usize = 20;
const NUM_DATA: usize = 2_000_000;

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
    let mut sum : u64 = *n;
    for i in 0..(1 << 25) {
        sum = sum + ((i ^ (i + *n)) >> 10);
    }
    thread::yield_now();
    sum
}

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    //let expect: u64 = vec.iter().map(load).sum();
    let mut rng: SmallRng = SmallRng::seed_from_u64(0xdead_beef);
    let vec = Arc::new(
        (0..NUM_DATA)
            .map(|_| rng.next_u32() as u64)
            .collect::<Vec<_>>(),
    );

    let timeout = api::ax_wait_timeout(&MAIN_WQ, Duration::from_millis(500));
    assert!(timeout);

    for i in 0..NUM_TASKS {
        let vec: Arc<Vec<u64>> = vec.clone();
        thread::spawn(move || {
            let start_time = std::time::Instant::now();
            let left = 0;
            let right = ((i % 4) * 4 + 1) as u64;
            println!(
                "part {}: {:?} [{}, {})",
                i,
                thread::current().id(),
                left,
                right
            );

            for j in left..right {
                RESULTS.lock()[i] += load(&vec[j as usize]);
            }
            LEAVE_TIME.lock()[i] = start_time.elapsed().as_millis() as u64;

            barrier();

            println!("part {}: {:?} finished", i, thread::current().id());
            let n = FINISHED_TASKS.fetch_add(1, Ordering::Relaxed);
            if n == NUM_TASKS - 1 {
                api::ax_wait_queue_wake(&MAIN_WQ, 1);
            }
        });
    }

    let timeout = api::ax_wait_timeout(&MAIN_WQ, Duration::from_millis(12000));
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
