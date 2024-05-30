功能实现
把sys_task_info函数给完善了，同时也完善了TaskControlBlock结构体，以及taskmanger的函数接口 刚开始那个inner总是出错，后来在taskmanger定义函数，然后在sys_task_info函数里面调用过来就好了 那个系统调用有点难度，听了别人的指点，我是想在syscall函数里面去加上每次的调用最终问题得到解决。 花了好几天看文档，自己看源代码然后自己写一个小模块都费劲 不过很有成就感。

简答作业
第 1 题
出错行为：
ch2b_bad_address: [kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003ac, kernel killed it.
ch2b_bad_instructions: [kernel] IllegalInstruction in application, kernel killed it.
ch2b_bad_register: [kernel] IllegalInstruction in application, kernel killed it.

rustsbi 版本：
[rustsbi] RustSBI version 0.3.0-alpha.2, adapting to RISC-V SBI v1.0.0
[rustsbi] Implementation     : RustSBI-QEMU Version 0.2.0-alpha.2
[rustsbi] Platform Name      : riscv-virtio,qemu

第 2 题
1.a0 代表 __restore 前一个函数的返回值，__restore 可用于在中断结束和系统调用结束时恢复用户寄存器状态。
2.上述代码特殊处理了三个寄存器，它们分别是 sstatus, sepc 和 sscratch，他们均用于存储系统中断的相关信息，用于在中断结束后正确返回用户代码。其中 sstatus 是系统状态寄存器，sepc 是程序计数器寄存器，sscratch 用于保存用户栈地址。
3.x2 是（用户）栈指针寄存器，该指针已被保存到 sscratch；x4 是线程指针寄存器，目前还尚未使用，因此不需要保存。
4.sp 将指向内核栈指针，sscratch 将指向用户栈指针。
5.状态切换发生在 csrw sstatus, t0 指令中，该指令将先前保存的（位于用户态的）系统状态信息从 t0 重新恢复到 sstatus，此时程序将从内核态回到用户态。
6.sp 将指向内核栈指针，sscratch 将指向用户栈指针。
7.从 U 态进入 S 态是在触发系统调用时，由处理器自动切换的

荣誉准则
1.在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

无

2.此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

无

3.我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4.我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。