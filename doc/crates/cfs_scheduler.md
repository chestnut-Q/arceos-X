# CFS Scheduler

CFS (Completely Fair Scheduler)，即完全公平调度器。

## 原理

为每个进程提供均等的 CPU 访问机会。使用红黑树来跟踪进程的虚拟运行时间，确保每个进程获得公平的 CPU 时间。

## 优点

公平，响应性好。

## 缺点

对于某些特定类型的负载，可能不如其他策略高效。

## 应用场景

Linux内核。