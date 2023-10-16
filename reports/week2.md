# Week 2 进展日志

## 选题

**题目**：ArceOS 的调度算法

**组员名单**：秦若愚

**目标描述**：调度算法在操作系统中扮演着至关重要的角色，它负责管理和分配系统资源，确保多个任务以高效和公平的方式运行。调度算法的作用包括任务优先级管理、进程间切换、资源分配和响应时间控制等，直接影响系统性能和用户体验。我将以开源操作系统ArceOS为基础，深入分析其任务调度模块。该工作的第一步是设计测试用例，评估ArceOS当前的调度算法性能，包括响应时间、吞吐量等指标。第二步是针对测试结果提出改进策略，优化ArceOS的调度算法，以提高系统整体性能和用户体验。

**代码仓库：** https://github.com/chestnut-Q/arceos-X

## 工作目标

1. 运行 arcesos，写工作日志
2. 参考叶昊星同学的工作，复现其测例以及实现的调度算法，完成接口 crate 文档
   1. https://github.com/131131yhx/arceos/tree/realloadbalance/report
   2. https://github.com/131131yhx/arceos/tree/main
3. 继续研究叶昊星同学的调度算法，实现优化

## 参考资料

1. ArceOS: https://github.com/rcore-os/arceos