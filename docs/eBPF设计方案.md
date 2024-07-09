# eBPF设计方案

## eBPF指令集

eBPF 有 10 个通用寄存器和一个只读帧指针寄存器，它们都是 64 位宽。

eBPF 调用约定定义如下：

> - R0：函数调用的返回值，以及 eBPF 程序的退出值
> - R1 – R5：函数调用的参数
> - R6 - R9：函数调用将保留的被调用者保存的寄存器
> - R10：用于访问堆栈的只读帧指针

R0 - R5 是临时寄存器，如果需要，eBPF 程序需要在调用过程中spill/fill它们。



## rbpf使用

rbpf是一个用户态的虚拟机，用于执行eBPF程序。现在其已经支持no_std环境。使用rbpf运行eBPF程序的步骤：

1. 创建虚拟机。虚拟机有多种类型。创建虚拟机时，将 eBPF 程序作为参数传递给构造函数。
2. 如果想要使用一些辅助函数，请将它们注册到虚拟机中。
3. 如果您想要一个 JIT 编译的程序，请编译它。
4. 执行您的程序：运行解释器或调用 JIT 编译函数。

rbpf中的不同类型的虚拟机：

需要注意的是，这些虚拟机定义底下的实现是一致的，只是针对不同的场景做了一个简单的封装。

- `struct EbpfVmMbuffer`模仿内核。当程序运行时，提供给其第一个 eBPF 寄存器的地址将是用户提供的元数据缓冲区的地址，并且预计该地址将包含指向数据包数据内存区域的开始和结束的指针。

> eBPF的初衷是用来过滤数据包，因此其第一个参数通常是指向数据包元数据的指针。
>
> 这个虚拟机使用的时候需要传入两个数据结构
>
> ```
> vm.execute_program(mem, mbuff)
> ```
>
> 其中第一个参数mem指向的是数据区域，mbuff指向的是元数据区域，mbuff的指针会作为第一个参数传入
>
> 在内部的执行过程中，读取数据的指令会访问mem内部的数据

- `struct EbpfVmFixedMbuff`有一个目的：使创建的程序能够与内核兼容，同时节省用户手动处理元数据缓冲区的工作量。实际上，此结构具有传递给程序的静态内部缓冲区。用户必须指示 eBPF 程序期望在缓冲区中找到数据包数据的开始和结束的偏移值。在调用运行程序的函数（无论是否经过 JIT 处理）时，该结构会自动更新此静态缓冲区中指定偏移量的地址，以获取程序调用的数据包数据的开始和结束。

> 与上一个虚拟机的使用的区别在于，这个虚拟机帮助在内部定义了元数据mbuff区域
>
> ```
>  rbpf::EbpfVmFixedMbuff::new(Some(prog), 0x40, 0x50)
> ```
>
> 其初始化过程需要指定元数据的内容，也就是数据的起始位置和结束位置
>
> 这两个虚拟机似乎是针对网络包处理的？？

- `struct EbpfVmRaw`适用于想要直接在数据包数据上运行的程序。不涉及任何元数据缓冲区，eBPF 程序直接在其第一个寄存器中接收数据包数据的地址。这是 uBPF 的行为。

> 这个虚拟机在执行程序前第一个参数指向的就是数据区域的指针

- `struct EbpfVmNoData`不接受任何数据。eBPF 程序不接受任何参数，并且其返回值是确定性的。不太确定这是否有有效的用例，但如果没有其他用途，这对于单元测试非常有用。

> 这个虚拟机不会设置数据区域的指针，所以访问数据会报错，主要用来做测试



**eBPF程序在内核中不只是服务于包过滤系统，其它很多地方也使用到，这些地方在调用eBPF程序时，通常会提供一个或多个参数，作为eBPF程序运行的上下文，当只有一个参数时，我们会使用`EbpfVmRaw` 这个虚拟机，当存在多个参数时，则该库没有提供对应的实现，这应该需要新增一些代码**。



以一个简单的程序，观察`EbpfVmRaw` 的用法：

```rust
let hkey = helpers::BPF_TRACE_PRINTK_IDX as u8;
let prog = &[
    0x85, 0x00, 0x00, 0x00, hkey, 0x00, 0x00, 0x00, // call helper <hkey>
    0x71, 0x10, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, // ldxh r0, [r1+2]
    0x95, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // exit
];
// Let's use some data.
let mem = &mut [0xaa, 0xbb, 0x11, 0xcc, 0xdd];
// This is an eBPF VM for programs reading from a given memory area (it
// directly reads from packet data)
let mut vm = rbpf::EbpfVmRaw::new(Some(prog)).unwrap();
vm.register_helper(hkey as u32, helpers::bpf_trace_printf).unwrap();
assert_eq!(vm.execute_program(mem).unwrap(), 0x11);
```

首先就是一段从第一个参数指向的数据中读取数据的程序，同时其还会调用eBPF定义的帮助函数，这里只是简单的使用rbpf中给出的默认实现，它只会打印调用输出帮助函数时的后三个参数，对于帮助函数的作用和用法，下文详细说明。在开始执行代码前，我们可以注册对应的帮助函数。

## eBPF in C

target: 编写一个C语言的eBPF程序，编译后使用rbpf来执行代码。

```c
#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>

SEC("xdp")
int hello(void *ctx) {
    bpf_printk("Hello World %d", 10);
    bpf_printk("xxxxx yyyyy");
    return XDP_PASS;
}

char LICENSE[] SEC("license") = "Dual BSD/GPL";
```

这个程序改自XDP的hello world程序，并使用clang 进行编译。查看其二进制文件的内容:

```
llvm-objdump -S hello.bpf.o

hello.bpf.o:    file format elf64-bpf

Disassembly of section xdp:

0000000000000000 <hello>:
; int hello(void *ctx) {
       0:       b7 01 00 00 00 00 00 00 r1 = 0
;     bpf_printk("Hello World %d", 10);
       1:       73 1a fe ff 00 00 00 00 *(u8 *)(r10 - 2) = r1
       2:       b7 01 00 00 25 64 00 00 r1 = 25637
       3:       6b 1a fc ff 00 00 00 00 *(u16 *)(r10 - 4) = r1
       4:       b7 01 00 00 72 6c 64 20 r1 = 543452274
       5:       63 1a f8 ff 00 00 00 00 *(u32 *)(r10 - 8) = r1
       6:       18 01 00 00 48 65 6c 6c 00 00 00 00 6f 20 57 6f r1 = 8022916924116329800 ll
       8:       7b 1a f0 ff 00 00 00 00 *(u64 *)(r10 - 16) = r1
       9:       bf a1 00 00 00 00 00 00 r1 = r10
      10:       07 01 00 00 f0 ff ff ff r1 += -16
;     bpf_printk("Hello World %d", 10);
      11:       b7 02 00 00 0f 00 00 00 r2 = 15
      12:       b7 03 00 00 0a 00 00 00 r3 = 10
      13:       85 00 00 00 06 00 00 00 call 6
      14:       b7 01 00 00 79 79 79 00 r1 = 7960953
;     bpf_printk("xxxxx yyyyy");
      15:       63 1a f8 ff 00 00 00 00 *(u32 *)(r10 - 8) = r1
      16:       18 01 00 00 78 78 78 78 00 00 00 00 78 20 79 79 r1 = 8753063052560595064 ll
      18:       7b 1a f0 ff 00 00 00 00 *(u64 *)(r10 - 16) = r1
      19:       bf a1 00 00 00 00 00 00 r1 = r10
      20:       07 01 00 00 f0 ff ff ff r1 += -16
;     bpf_printk("xxxxx yyyyy");
      21:       b7 02 00 00 0c 00 00 00 r2 = 12
      22:       85 00 00 00 06 00 00 00 call 6
;     return XDP_PASS;
      23:       b7 00 00 00 02 00 00 00 r0 = 2
      24:       95 00 00 00 00 00 00 00 exit
```

- `SEC("xdp")` 定义了这段程序放在xdp段，我们可以随机指定段的名称，但对于一些用户态工具，他们会工具段的名称判断程序是否可挂载到某些内核挂载点上。
- 程序中我们只是简单打印了两行字符串，从反汇编的代码来看，程序只是往堆栈上写了一些数据后就执行call 指令调用函数
  - 这些往堆栈上写数据的过程其实就是在把打印的字符串内容写到堆栈上
    - 这些字符串在.rodata区域也有保存，从https://nakryiko.com/posts/bpf-tips-printk/ 这篇文章中可以知道，在新的系统上，eBPF程序不会使用这种低效的手段打印字符，它会从.rodata区域寻找这个字符串
  - eBPF的call 指令会以一个编号作为参数，这个编号就是eBPF帮助函数的编号，也就是上文我们做帮助函数注册使用的编号

要想运行这一段程序，肯定不能直接运行这个文件。从libbpf库或者Aya工具链的实现来看，在用户态，需要有工具解析编译出来的可执行文件，从里面找到eBPF程序段，并通过系统调用注册到内核当中。对于更复杂的内容，比如全局变量，数组等内容，也需要用户态程序通过系统调用请求内核实现。

这里我们只是简单地尝试运行这里的eBPF程序段，因此我们直接使用现有的`elf` 库解析ELF文件，找到xdp段，读出这里的二进制代码，并用rbpf虚拟机运行。

```rust
let filename = PathBuf::from("./libbpf/bpf/hello.bpf.o");
let file_data = std::fs::read(filename).expect("Could not read file.");
let slice = file_data.as_slice();
let file = ElfBytes::<AnyEndian>::minimal_parse(slice).expect("Open test1");
// Get the ELF file's build-id
let xdp: SectionHeader = file
    .section_header_by_name("xdp")
    .expect("section table should be parseable")
    .expect("file should have a xdp section");
let (data,_) = file.section_data(&xdp).unwrap();
let prog = data.to_vec();
let mut vm = rbpf::EbpfVmRaw::new(Some(&prog)).unwrap();
vm.register_helper(hkey as u32, trace_printf).unwrap();
let res = vm.execute_program(&mut []).unwrap();
println!("Program returned: {res:?} ({res:#x})");

struct FakeOut;
impl Write for FakeOut {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        print!("{}", s);
        Ok(())
    }
}


pub fn trace_printf (fmt_ptr: u64, _fmt_len: u64, arg3: u64, arg4: u64, arg5: u64) -> u64 {
    // println!("bpf_trace_printf: {fmt_ptr:#x} {fmt_len:#x} {arg3:#x}, {arg4:#x}, {arg5:#x}");
    unsafe {
        printf_with(&mut FakeOut, fmt_ptr as _ , arg3, arg4, arg5) as u64
    }
}
```

这里的逻辑比较简单，不再赘述，主要关注的是`bpf_trace_printk` 的实现，在c实现中，我们使用`bpf_printk` 打印内容，这只是`bpf_trace_printk`的封装，而eBPF代码执行时，会调用注册好的实现。在rbpf中，这些帮助函数使用一个map保存。

rbpg默认提供的实现并不会处理真正的打印，在内核中，我们需要实现真正的printf，为了在rust中实现printf，我们需要开启一个特定的feature `#![feature(c_variadic)]` 。同时在借助printf-compat 库的功能，我们就能实现一个简陋版本的打印函数。

```rust
/// Printf according to the format string, function will return the number of bytes written(including '\0')
pub unsafe extern "C" fn printf(w: &mut impl Write,str: *const c_char, mut args: ...) -> c_int {
    let bytes_written = format(str, args.as_va_list(), output::fmt_write(w));
    bytes_written + 1
}

/// Printf with '\n' at the end, function will return the number of bytes written(including '\n' and '\0')
pub unsafe extern "C" fn printf_with(w: &mut impl Write, str: *const c_char, mut args: ...) -> c_int {
    let str = core::ffi::CStr::from_ptr(str).to_str()
        .unwrap()
        .as_bytes();
    let bytes_written = if str.ends_with(b"\n") {
        format(str.as_ptr() as _, args.as_va_list(), output::fmt_write(w))
    } else {
        let mut bytes_written = format(str.as_ptr() as _, args.as_va_list(), output::fmt_write(w));
        w.write_str("\n").unwrap();
        bytes_written += 1;
        bytes_written
    };
    bytes_written + 1
}
```

通过上面的实现，就可以正确运行这个程序，并输出结果:

```
Hello World 10
xxxxx yyyyy
Program returned: 2 (0x2)
```





## windows 的ebpf移植方案

https://github.com/microsoft/ebpf-for-windows?tab=readme-ov-file

![架构概述](./assert/ArchitectureDiagram.png)

[PREVAIL](https://github.com/vbpf/ebpf-verifier?tab=readme-ov-file)  ebpf验证器，windows将其用在用户态对ebpf程序进行检查

[ubpf](https://github.com/iovisor/ubpf) 使用ubpf进行JIT编译，并在内核态执行





## reference

https://docs.kernel.org/bpf/ linux kernel对ebpf的介绍，很多信息位于这里

https://docs.cilium.io/en/latest/bpf/ xdp中对ebpf的介绍部分也很详细

https://terenceli.github.io/%E6%8A%80%E6%9C%AF/2020/08/09/ebpf-with-tracepoint tracepoint如何和ebpf程序结合