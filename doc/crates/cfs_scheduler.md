# Completely Fair Scheduler (CFS) Overview

The Completely Fair Scheduler (CFS) is Linux's default process and task scheduler. Its primary aim is to ensure all processes receive an equitable share of the processor's time. Below is a concise overview of its mechanisms:

## Virtual Runtime (vruntime)

Represents the amount of time a process has run. Processes with smaller vruntime values are scheduled first, meaning infrequently run processes get scheduled promptly when they become active.

## Red-Black Tree

CFS employs a red-black tree to track all runnable processes, sorted by vruntime. The leftmost node (smallest vruntime) is scheduled first.

## Dynamic Time Slices

CFS doesn't have fixed time quanta. Instead, it calculates dynamic time slices based on system load and task demands.

## Sleep and Wake-up

Processes that sleep for extended periods don't accrue vruntime, allowing them to be scheduled quickly upon waking.

## Task Weights and Scheduling Groups

CFS permits setting task weights. Higher-weighted tasks receive more CPU time. Groups of tasks can be managed together through scheduling groups.

## Load Balancing

In multi-processor systems, CFS ensures tasks are evenly distributed among all processors.

The goal of CFS is to deliver fair CPU time to every process while maintaining high throughput and responsiveness.