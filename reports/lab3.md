功能实现

本实验实现了 sys_spawn 系统调用和 Stride 调度算法。sys_spawn 系统调用通过为指定的 ELF 文件创建一个新的 TaskControlBlock，并在新的地址空间中为其分配内存。

简答作业

1,下一个要执行的不是 p1，而是 p2。这是因为 p2 的 stride 值出现了溢出，应该是 260，但实际变成了 5。因此，调度算法选择执行 p2。

2,在所有进程的优先级都大于等于 2 的情况下，根据公式 pass = BigStride / priority，我们可以得出 pass 的值大于 BigStride 的一半。

3,代码如下

use core::cmp::Ordering;

struct Stride(u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if other.0 > BIG_STRIDE / 2 {
            return Some(Ordering::Less);
        } 
        if self.0 > BIG_STRIDE / 2 {
            return Some(Ordering::Greater);
        }
        self.0.partial_cmp(&other.0)
    }
}

impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        self.partial_cmp(other).is_some_and(|it| it == Ordering::Equal)
    }
}

荣誉准则

1.在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

无

2.此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

无

3.我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4.我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。