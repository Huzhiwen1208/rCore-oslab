## 简单总结实现的功能
在实验lab3(ch5)中，实现了新的系统调用`sys_spawn`，并实现了基于`stride`任务调度算法。其中`sys_spawn`的功能是，在给定可执行文件ELF路径情况下，可以基于该ELF文件生成一个子进程。类似于fork+exec，但是不同的是spawn不需要复制父进程的内存空间，而是直接基于文件内容生成内存空间。
## 问答作业
1. stride 算法深入
> 1.1 stride 算法原理非常简单，但是有一个比较大的问题。例如两个 pass = 10 的进程，使用 8bit 无符号整形储存 stride， p1.stride = 255, p2.stride = 250，在 p2 执行一个时间片后，理论上下一次应该 p1 执行。实际情况是轮到 p1 执行吗？为什么？
> &nbsp;&nbsp;&nbsp;&nbsp;答：不是！因为8bit无符号数情况下的取值范围是0~255。p2执行一个时间片后，p2.stride=250+10=260，会溢出为4。所以下一次还是p2执行。

> 1.2 我们之前要求进程优先级 >= 2 其实就是为了解决这个问题。可以证明， 在不考虑溢出的情况下 , 在进程优先级全部 >= 2 的情况下，如果严格按照算法执行，那么 STRIDE_MAX – STRIDE_MIN <= BigStride / 2。为什么？尝试简单说明（不要求严格证明）。
> &nbsp;&nbsp;&nbsp;&nbsp;答：可以看出优先级>=2时，步长pass<=BigStride/2。而假设STRIDE_MAX - STRIDE_MIN > BigStride/2时，我们可以发现最小stride进程和最大stride进程的差距已经大于一个最大的pass（BigStride/2）。所以当最大stride进程在上一次执行前，其stride0已经是大于最小stride的状态了，所以不可能到达这一个时刻，也即是不可能存在STRIDE_MAX - STRIDE_MIN > BigStride/2。所以一定是STRIDE_MAX – STRIDE_MIN <= BigStride / 2。

> 1.3 已知以上结论，考虑溢出的情况下，可以为 Stride 设计特别的比较器，让 BinaryHeap<Stride> 的 pop 方法能返回真正最小的 Stride。补全下列代码中的 partial_cmp 函数，假设两个 Stride 永远不会相等。
``` rust
use core::cmp::Ordering;
struct Stride(u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // implement
        if self.eq(other) {
            // should not equal
            panic!("stride value should not equal");
            // return Some(Ordering::Equal);
        }

        if (self.0 > other.0 && self.0 - other.0 <= BigStride / 2) || 
           (self.0 < other.0 && other.0 - self.0 <= BigStride / 2)
        {
            return Ordering::Greater;
        }

        return Ordering::Less;
    }
}

impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}
```


## 荣誉准则
1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：无。
2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：无。
3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。
4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。