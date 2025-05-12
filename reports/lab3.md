# 总结

`sys_get_time` `sys_mmap` `sys_munmap` 实现方法和lab2类似，sys_munmap增加了对页表项的处理。此外删除了一些不必要的读写权限检测。

`sys_spawn`的实现流程和fork差不多，主要区别是程序的memory_set和内核栈等不是复制的父进程，而是从elf文件读取或重新创建。

Stride调度主要在tcb内部添加了stride和pass两个元素，pass通过`sys_set_priority`更新，stride在从TaskManager获取任务时更新。


# 简答题

1. stride 算法深入

    stride 算法原理非常简单，但是有一个比较大的问题。例如两个 pass = 10 的进程，使用 8bit 无符号整形储存 stride， p1.stride = 255, p2.stride = 250，在 p2 执行一个时间片后，理论上下一次应该 p1 执行。

    实际情况是轮到 p1 执行吗？为什么？

        不是，p1+10后移除，实际值为9

    我们之前要求进程优先级 >= 2 其实就是为了解决这个问题。可以证明， 在不考虑溢出的情况下 , 在进程优先级全部 >= 2 的情况下，如果严格按照算法执行，那么 STRIDE_MAX – STRIDE_MIN <= BigStride / 2。  

    为什么？尝试简单说明（不要求严格证明）。

        考虑极限增量情况优先级=2，此时更新Stride增量为BigStride / 2，如果要使更新后STRIDE_MAX_t-STRIDE_MIN_t > BigStride / 2 ，要求STRIDE_MAX_t-1 > STRIDE_MIN_t-1,但这与Stride调度的选择方法冲突，因此不成立。

    已知以上结论，考虑溢出的情况下，可以为 Stride 设计特别的比较器，让 BinaryHeap<Stride> 的 pop 方法能返回真正最小的 Stride。补全下列代码中的 partial_cmp 函数，假设两个 Stride 永远不会相等。
    
    ```
    use core::cmp::Ordering;

    struct Stride(u8); //这里是不是改成u8?
    
    const THRESHOLD: u8 = u8::MAX / 2;

    impl PartialOrd for Stride {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            if (self.0 < other.0 && other.0 - self.0 <= THRESHOLD) ||
                (self.0 > other.0 && self.0 - other.0 >= THRESHOLD) {
                    Some(Ordering::Less)
                }
         } else {
            Some(Ordering::Greater)
         }
    }

    impl PartialEq for Stride {
        fn eq(&self, other: &Self) -> bool {
            false
        }
    }

    ```
# 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

    > 暂无
2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

    > rCore-Tutorial-Book-v3 3.6.0-alpha.1

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
