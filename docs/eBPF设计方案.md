# eBPF设计方案

## eBPF指令集

### 寄存器

eBPF 有 10 个通用寄存器和一个只读帧指针寄存器，它们都是 64 位宽。

eBPF 调用约定定义如下：

> - R0：函数调用的返回值，以及 eBPF 程序的退出值
> - R1 – R5：函数调用的参数
> - R6 - R9：函数调用将保留的被调用者保存的寄存器
> - R10：用于访问堆栈的只读帧指针

R0 - R5 是临时寄存器，如果需要，eBPF 程序需要在调用过程中spill/fill它们。



指令编码：

1. 基本指令编码，使用 64 位对指令进行编码
2. 宽指令编码，在基本指令后附加第二个64位，总共128位

### 基本指令编码

```
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|    opcode     |     regs      |            offset             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                              imm                              |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

其对应的长度和字节序如下所示:

| 32 bits (MSB) | 16 bits | 4 bits          | 4 bits               | 8 bits (LSB) |
| ------------- | ------- | --------------- | -------------------- | ------------ |
| immediate     | offset  | source register | destination register | opcode       |

操作码字段：

```
+-+-+-+-+-+-+-+-+
|specific |class|
+-+-+-+-+-+-+-+-+
```

- **specific**: 这些位的格式因指令类别而异
- **class**: 指令的类别

寄存器字段：(小端机器上)

```
+-+-+-+-+-+-+-+-+
|src_reg|dst_reg|
+-+-+-+-+-+-+-+-+
```

- src_reg: 源寄存器编号（0-10）64位立即数指令中有其它作用
- dst_reg: 目标寄存器编号（0-10）

offset字段：与指针算法一起使用的有符号整数偏移量，除非另有说明（某些算术指令将此字段重用于其他目的）

### 宽指令编码

一些指令被定义为使用宽指令编码，该编码使用两个 32 位立即数。基本指令格式后面的 64 位包含一个伪指令，其中 'opcode'、'dst_reg'、'src_reg' 和 'offset' 均设置为零。

```
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|    opcode     |     regs      |            offset             |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                              imm                              |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                           reserved                            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                           next_imm                            |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

- **reserved** : unused, set to zero
- **next_imm**: second signed integer immediate value

### [Load and store instructions](https://www.kernel.org/doc/html/latest/bpf/standardization/instruction-set.html#id19)

对于`LD`, `LDX`, `ST`, and `STX` 这几个加载和存储指令，8字节的操作码编码如下:

```
+-+-+-+-+-+-+-+-+
|mode |sz |class|
+-+-+-+-+-+-+-+-+
```

mode字段有以下属性：

| mode modifier | value | description                         | reference                                                    |
| :------------ | :---- | :---------------------------------- | :----------------------------------------------------------- |
| IMM           | 0     | 64-bit immediate instructions       | [64-bit immediate instructions](https://www.kernel.org/doc/html/latest/bpf/standardization/instruction-set.html#bit-immediate-instructions) |
| ABS           | 1     | legacy BPF packet access (absolute) | [Legacy BPF Packet access instructions](https://www.kernel.org/doc/html/latest/bpf/standardization/instruction-set.html#legacy-bpf-packet-access-instructions) |
| IND           | 2     | legacy BPF packet access (indirect) | [Legacy BPF Packet access instructions](https://www.kernel.org/doc/html/latest/bpf/standardization/instruction-set.html#legacy-bpf-packet-access-instructions) |
| MEM           | 3     | regular load and store operations   | [Regular load and store operations](https://www.kernel.org/doc/html/latest/bpf/standardization/instruction-set.html#regular-load-and-store-operations) |
| MEMSX         | 4     | sign-extension load operations      | [Sign-extension load operations](https://www.kernel.org/doc/html/latest/bpf/standardization/instruction-set.html#sign-extension-load-operations) |
| ATOMIC        | 6     | atomic operations                   | [Atomic operations](https://www.kernel.org/doc/html/latest/bpf/standardization/instruction-set.html#atomic-operations) |

size 字段有以下属性：

| size | value | description           |
| :--- | :---- | :-------------------- |
| W    | 0     | word (4 bytes)        |
| H    | 1     | half word (2 bytes)   |
| B    | 2     | byte                  |
| DW   | 3     | double word (8 bytes) |

##### IMM mode 

> 带有 IMM mode 修饰符的指令使用指令编码中定义的宽指令编码，并使用基本指令的 ‘src_reg’ 字段来保存操作码子类型。下表使用“src_reg”字段中的操作码子类型定义了一组{IMM，DW，LD}指令, 这些指令在执行时被赋予特定的功能

| src_reg | pseudocode                                | imm type    | dst type     |
| :------ | :---------------------------------------- | :---------- | :----------- |
| 0x0     | dst = (next_imm << 32) \| imm             | integer     | integer      |
| 0x1     | dst = map_by_fd(imm)                      | map fd      | map          |
| 0x2     | dst = map_val(map_by_fd(imm)) + next_imm  | map fd      | data address |
| 0x3     | dst = var_addr(imm)                       | variable id | data address |
| 0x4     | dst = code_addr(imm)                      | integer     | code address |
| 0x5     | dst = map_by_idx(imm)                     | map index   | map          |
| 0x6     | dst = map_val(map_by_idx(imm)) + next_imm | map index   | data address |

- map_by_fd(imm) 表示将 32 位文件描述符转换为映射(map)的地址
- map_by_idx(imm) 表示将32位索引转换为map的地址
- map_val(map) 获取给定映射(map)中第一个值的地址
- var_addr(imm) 获取具有给定 id 的平台变量(platform variable)的地址
- code_addr(imm) 获取（64 位）指令数中指定相对偏移处的指令地址
- 反汇编程序可以使用“imm type”进行显示
- dst 类型可用于验证和 JIT 编译

##### maps

映射是某些平台上的 BPF 程序可访问的共享内存区域。映射可以具有单独文档中定义的各种语义，并且可能有也可能没有单个连续的内存区域，但`map_val(map)`目前仅针对具有单个连续内存区域的映射进行定义

如果平台支持，每个映射都可以有一个文件描述符 (fd)，其中 `map_by_fd(imm)` 表示获取具有指定文件描述符的映射。每个 BPF 程序也可以定义为在加载时使用与程序关联的一组映射，而 `map_by_idx(imm)` 表示获取与包含指令的 BPF 程序关联的集合中具有给定索引的映射

##### platform variable

平台变量是内存区域，由整数 ID 标识，由运行时公开，并可由某些平台上的 BPF 程序访问。'var_addr(imm)' 操作表示获取由给定 ID 标识的内存区域的地址



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



## eBPF 全局变量

上文展示的程序中没有全局变量的访问，较为简单。这一节我们使用一个较为复杂的例子展示从用户态到内核态运行一个eBPF程序还需要哪些步骤。这个程序的C语言版本如下：

```c
int counter = 0;
int counter2 = 1;
SEC("xdp")
int hello(void *ctx) {
    counter++;
    counter2++;
    return XDP_PASS;
}
```

程序中声明了两个变量，一个初始化为0，一个为1。按照对程序编译的理解，这两个变量会被保存到bss段和data段中。使用`llvm-readelf -S` 命令查看文件的段信息：

```
[ 3] xdp               PROGBITS        0000000000000000 000040 000060 00  AX  0   0  8
[ 4] .relxdp           REL             0000000000000000 0007d0 000020 10   I 26   3  8
[ 5] .bss              NOBITS          0000000000000000 0000a0 000004 00  WA  0   0  4
[ 6] .data             PROGBITS        0000000000000000 0000a0 000004 00  WA  0   0  4
```

其中可以看到确实存在bss段和data段，并且大小为4，符合我们在程序中声明的变量。这里多出来的`.relxdp`段很重要，稍后我们再解释它。

将程序反汇编，观察一下对应的汇编指令：`llvm-objdump -dr`

```
hello.bpf.o:    file format elf64-bpf

Disassembly of section xdp:

0000000000000000 <hello>:
       0:       18 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 r1 = 0 ll
                0000000000000000:  R_BPF_64_64  counter
       2:       61 12 00 00 00 00 00 00 r2 = *(u32 *)(r1 + 0)
       3:       07 02 00 00 01 00 00 00 r2 += 1
       4:       63 21 00 00 00 00 00 00 *(u32 *)(r1 + 0) = r2
       5:       18 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 r1 = 0 ll
                0000000000000028:  R_BPF_64_64  counter2
       7:       61 12 00 00 00 00 00 00 r2 = *(u32 *)(r1 + 0)
       8:       07 02 00 00 01 00 00 00 r2 += 1
       9:       63 21 00 00 00 00 00 00 *(u32 *)(r1 + 0) = r2
      10:       b7 00 00 00 02 00 00 00 r0 = 2
      11:       95 00 00 00 00 00 00 00 exit
```

这些汇编指令都很简单，但第0行和第5行的指令与其它的不一样，它的长度是16个字节的，并且从程序逻辑来看，这两条指令的作用应该是**将我们定义的变量的地址加载到寄存器中**。 但是这里显示其将0加载到了寄存器当中，这显然不太正确。**其下方的重定位项显示了这条指令在运行时需要经过重定位**。



重定位的目的是确定每个符号定义的运行时内存地址，并修改对这些符号的引用，使之指向符号定义的运行时内存地址。

重定位的整体过程可以分为两个步骤：

- 重定位节和符号定义。链接器将输入目标文件的相同节合并成一个节，合并的节将作为可执行目标文件中此类型的节。随后，链接器确定每个合并节的运行时内存地址，并确定合并节中符号定义的运行时内存地址。这一步骤完成后，可执行目标文件中的所有指令和符号定义的运行时内存地址就唯一确定了。
- 重定位节中的符号引用。链接器修改所有的符号引用，使之指向符号定义的运行时内存地址。链接器要执行此步骤依赖于目标文件中的重定位信息。

重定位有几种类型：

- 链接时重定位
- 装载时重定位
- 地址无关代码（PIC）

eBPF程序与C语言类似，也有重定位项，这些重定位项应该属于装载时重定位。

https://docs.kernel.org/bpf/llvm_reloc.html

https://www.kernel.org/doc/html/v5.17/bpf/llvm_reloc.html

eBPF使用的重定位项格式如下:

```
typedef struct
{
  Elf64_Addr    r_offset;  // Offset from the beginning of section.
  Elf64_Xword   r_info;    // Relocation type and symbol index.
} Elf64_Rel;
```

其重定位类型列表如下所示：

```
Enum  ELF Reloc Type     Description      BitSize  Offset        Calculation
0     R_BPF_NONE         None
1     R_BPF_64_64        ld_imm64 insn    32       r_offset + 4  S + A
2     R_BPF_64_ABS64     normal data      64       r_offset      S + A
3     R_BPF_64_ABS32     normal data      32       r_offset      S + A
4     R_BPF_64_NODYLD32  .BTF[.ext] data  32       r_offset      S + A
10    R_BPF_64_32        call insn        32       r_offset + 4  (S + A) / 8 - 1
```

 查看刚才编译的文件的重定位段` .relxdp`, 其内容如下:

```
Relocation section '.relxdp' at offset 0x7d0 contains 2 entries:
    Offset             Info             Type               Symbol's Value  Symbol's Name
0000000000000000  0000000b00000001 R_BPF_64_64            0000000000000000 counter
0000000000000028  0000000c00000001 R_BPF_64_64            0000000000000000 counter2
```

可以看到这两个重定位项正好对应上文反汇编代码中的信息。

现在想要直接使用rbpf来运行这个程序就不行了，因为rbpf不处理重定位的过程，直接执行指令将会导致内存访问错误。

看到这里，如果没有已有的解决方案，一种可能的解决方法是用户态程序可以为bss段和data段在内核中创建一个内存映射，然后根据重定位项的信息修改指令，将对应的映射地址填入指令中。

Linux kernel对此的解决方法是使用`MAP`。BPF Map本质上是以「键/值」方式存储在内核中的数据结构，它们可以被任何知道它们的BPF程序访问。在内核空间的程序创建BPF Map并返回对应的文件描述符，在用户空间运行的程序就可以通过这个文件描述符来访问并操作BPF Map，这就是为什么BPF Map在BPF世界中是桥梁的存在了。

https://davidlovezoe.club/wordpress/archives/1044?ref=edony.ink

https://docs.cilium.io/en/stable/bpf/architecture/#maps  

为什么内核使用 Map而不是内存映射：

1. eBPF程序运行在内核内，如果使用内存映射，意味着当程序退出，映射也会消失，但很多eBPF程序的生命周期要长于用户程序
2. 内存映射不能在多个eBPF程序之间共享内容

有了Map这个基础设施后，我们就可以在编写eBPF程序时声明一个map来保存eBPF运行过程产生的信息，比如下面的代码：

```c
struct user_msg_t {
   char message[12];
};
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 10240);
    __type(key, u32);
    __type(value, struct user_msg_t);
} my_config SEC(".maps");
```

这会在.maps段定义一个map，其键是u32类型的数字，而值是一个字符数组，并且定义了其长度位10240。这是一种显式的定义map的方式。当我们编写的eBPF程序包含常见的全局变量的时候，这些变量也会被处理为map类型。下面我们就来看常见的用户态bpf库是如何处理ebpf程序的。



### Aya库探索

这里选择rust社区的Aya库进行探索事件，因为libbpf中的处理过于复杂，而且是由C语言实现的，逻辑不是那么明显。

Aya 是一个 eBPF 库，专注于可操作性和开发人员体验。它不依赖于libbpf或bcc - 它是从头开始完全用 Rust 构建的，仅使用libc crate 来执行系统调用。借助 BTF 支持并与 musl 链接，它提供了真正的一次编译，随处运行的解决方案，其中单个自包含的二进制文件可以部署在许多 Linux 发行版和内核版本上。

我们编写一个逻辑与上文展示的c代码相同的eBPF程序：

```rust
static mut MASK: usize = 1;
static mut MASK2: usize = 0;
#[kprobe]
pub fn myapp(ctx: ProbeContext) -> u32 {
    unsafe {
        MASK += 1;
        MASK2 +=1; 
    }
    0
}
```

其反汇编结果也与之前的类似：

```
0000000000000000 <myapp>:
       0:       18 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 r1 = 0 ll
                0000000000000000:  R_BPF_64_64  .data
       2:       79 12 00 00 00 00 00 00 r2 = *(u64 *)(r1 + 0)
       3:       07 02 00 00 01 00 00 00 r2 += 1
       4:       7b 21 00 00 00 00 00 00 *(u64 *)(r1 + 0) = r2
       5:       18 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 r1 = 0 ll
                0000000000000028:  R_BPF_64_64  .bss
       7:       79 12 00 00 00 00 00 00 r2 = *(u64 *)(r1 + 0)
       8:       07 02 00 00 01 00 00 00 r2 += 1
       9:       7b 21 00 00 00 00 00 00 *(u64 *)(r1 + 0) = r2
      10:       b7 00 00 00 00 00 00 00 r0 = 0
      11:       95 00 00 00 00 00 00 00 exit
```

这个程序并不能直接运行，因为我的内核似乎不支持bss段的重定位，内核验证器给出的错误是：

```
Error: the BPF_PROG_LOAD syscall failed. Verifier output: 0: R1=ctx(off=0,imm=0) R10=fp0
0: (18) r1 = 0xffff931141efad10       ; R1_w=map_value(off=0,ks=4,vs=8,imm=0)
2: (79) r2 = *(u64 *)(r1 +0)          ; R1_w=map_value(off=0,ks=4,vs=8,imm=0) R2_w=scalar()
3: (07) r2 += 1                       ; R2_w=scalar()
4: (7b) *(u64 *)(r1 +0) = r2          ; R1_w=map_value(off=0,ks=4,vs=8,imm=0) R2_w=scalar()
5: (18) r1 = 0xffff931141efa800       ; R1_w=map_ptr(off=0,ks=4,vs=8,imm=0)
7: (79) r2 = *(u64 *)(r1 +0)          ; R1_w=map_ptr(off=0,ks=4,vs=8,imm=0) R2_w=ptr_bpf_map_ops(off=0,imm=0)
8: (07) r2 += 1                       ; R2_w=ptr_bpf_map_ops(off=1,imm=0)
9: (7b) *(u64 *)(r1 +0) = r2
only read from bpf_array is supported
```

为了观察eBPF代码在被库处理后加载到内核之前其发生了什么变化，我们在用户态加载程序中打上一些注释，这里我们在Aya加载eBPF程序后，再次反汇编程序：

```rust
let mut bpf = Bpf::load(include_bytes_aligned!(
        "../../target/bpfel-unknown-none/release/myapp"
    ))?;

if let Err(e) = BpfLogger::init(&mut bpf) {
    // This can happen if you remove all log statements from your eBPF program.
    warn!("failed to initialize eBPF logger: {}", e);
}

let program: &mut KProbe = bpf.program_mut("myapp").unwrap().try_into()?;
let code = program.inst()?;
info!("code len: {}", code.len());
let prog = unsafe { core::slice::from_raw_parts(code.as_ptr() as *const u8, code.len() * 8) };
disassembler::disassemble(&prog);

program.load()?;
program.attach("try_to_wake_up", 0)?;
```

这里需要侵入到Aya库中做一些修改，因此拉取Aya库到本地并替换依赖，其结果如下：

```
lddw r1, 0xa
ldxdw r2, [r1+0x0]
add64 r2, 0x1
stxdw [r1+0x0], r2
lddw r1, 0xb
ldxdw r2, [r1+0x0]
add64 r2, 0x1
stxdw [r1+0x0], r2
mov64 r0, 0x0
exit
```

这里的汇编代码与上面的差异在于第一行和第5行，也就是那两条加载全局变量地址的指令。可以发现在Aya库处理之后，这里的加载到寄存器的内容变成了不同的值。我们需要知道**这个值代表着什么**？而且结合内核的错误来看，在内核中，程序还会被再次修改，因为内核的信息显式加载到寄存器的内容应该是一个指针。

为了找出这两个值是什么，需要理解Aya在处理eBPF程序时做了什么处理，通过阅读其源代码，并输出一些信息，我们大致确定了这个修改发生的位置以及相关的处理。

其路径大致如下：

```
Bpf::load
->EbpfLoader.load
	->MapData::create // 创建映射
	->obj.relocate_maps
		->relocate_maps // 修改指令
```

在`MapData::create`中，Aya为.data/.bss创建了两个map，打印的内容如下：

```
[2024-07-12T11:28:00Z ERROR aya::sys::bpf] bpf_create_map, name: ".data", map_type: 2, key_size: 4, value_size: 8, max_entries: 1
[2024-07-12T11:28:00Z WARN  aya::maps] created map with fd: OwnedFd { fd: 10 }
[2024-07-12T11:28:00Z ERROR aya::maps] map data is not empty, but section kind is not BSS, Data
[2024-07-12T11:28:00Z ERROR aya::sys::bpf] bpf_create_map, name: ".bss", map_type: 2, key_size: 4, value_size: 8, max_entries: 1
[2024-07-12T11:28:00Z WARN  aya::maps] created map with fd: OwnedFd { fd: 11 }
```

当然，Aya还创建了一些其它map，暂时不用管。

在`relocate_maps` 中，Aya会根据重定位段中的内容修改指令，这里就是修改加载全局变量的的指令，其核心的逻辑如下所示:

```rust
if !map.data().is_empty() {
    log::error!("relocate_maps: map data is not empty, set src_reg to BPF_PSEUDO_MAP_VALUE: {}", BPF_PSEUDO_MAP_VALUE);
    instructions[ins_index].set_src_reg(BPF_PSEUDO_MAP_VALUE as u8);
    instructions[ins_index + 1].imm = instructions[ins_index].imm + sym.address as i32;
} else {
    log::error!("relocate_maps: map data is empty, set src_reg to BPF_PSEUDO_MAP_FD: {}", BPF_PSEUDO_MAP_FD);
    instructions[ins_index].set_src_reg(BPF_PSEUDO_MAP_FD as u8);
}
log::error!("relocate_maps: set imm to fd: {}", fd);
instructions[ins_index].imm = *fd;
```

可以看到这里根据map中是否有数据做了不同对指令的修改，并且最后一个修改都是将指令的立即数改为一个文件描述符，这里的打印信息如下：

```
[2024-07-12T11:28:00Z ERROR aya_obj::relocation] relocate_maps: map data is not empty, set src_reg to BPF_PSEUDO_MAP_VALUE: 2
[2024-07-12T11:28:00Z ERROR aya_obj::relocation] relocate_maps: set imm to fd: 10
[2024-07-12T11:28:00Z ERROR aya_obj::relocation] relocate_maps: map data is empty, set src_reg to BPF_PSEUDO_MAP_FD: 1
[2024-07-12T11:28:00Z ERROR aya_obj::relocation] relocate_maps: set imm to fd: 11
```

可以看到，这个设置的文件描述符就是上面为.data/.bss创建的。

这里对指令的修改还包含了设置src寄存器的值和下一条指令的立即数部分，这个我们稍后再进行分析。

**综上，在用户态的bpf库中，对eBPF程序的加载不是直接的，必须经过额外的处理才能最终加载到内核中。**



### linux 探索

在上面，Aya库对eBPF程序经过重定位处理后，加载到了内核中，但内核的检查器给出的结果显式，内核对这个程序又做了处理。我们来看看内核做了什么处理？

相关的文档位于https://www.kernel.org/doc/html/latest/bpf/standardization/instruction-set.html#bit-immediate-instructions ， 这里给出了eBPF指令集的信息，其中包含了上文中被修改的LDDW 指令的内容。

对于LD系列的指令，src_reg字段被用做特殊用途，本文第一段对eBPF指令集的介绍已经翻译了Linux对这一部分的处理。[IMM Mode](#IMM mode)

现在我们可以根据LD指令的设计解释Aya中对指令的修改过程了, 

```rust
if !map.data().is_empty() {
    log::error!("relocate_maps: map data is not empty, set src_reg to BPF_PSEUDO_MAP_VALUE: {}", BPF_PSEUDO_MAP_VALUE);
    instructions[ins_index].set_src_reg(BPF_PSEUDO_MAP_VALUE as u8);
    instructions[ins_index + 1].imm = instructions[ins_index].imm + sym.address as i32;
} else {
    log::error!("relocate_maps: map data is empty, set src_reg to BPF_PSEUDO_MAP_FD: {}", BPF_PSEUDO_MAP_FD);
    instructions[ins_index].set_src_reg(BPF_PSEUDO_MAP_FD as u8);
}
log::error!("relocate_maps: set imm to fd: {}", fd);
instructions[ins_index].imm = *fd;
```

1. 当map中有数据时

   1. 设置LD指令的src_reg 为BPF_PSEUDO_MAP_VALUE ，这个值为2

   2. 因为这是一条宽指令，设置下一条指令的立即数部分为当前指令的立即数 + 符号的地址

      > 因为当前指令的立即数还没有被修改，所以这里当前指令的立即数应该是代表其它内容，比如偏移量什么的

2. 当map中没有数据

   1. 修改LD指令的src_reg为BPF_PSEUDO_MAP_FD，这个值为为1

3. 修改LD指令的立即数为创建好的map对应的文件描述符

结合我们代码中的内容和输出的内容来看：

```rust
static mut MASK: usize = 1;
static mut MASK2: usize = 0;
#[kprobe]
pub fn myapp(ctx: ProbeContext) -> u32 {
    unsafe {
        MASK += 1;
        MASK2 +=1; 
    }
    0
}
```

1. BSS段的内容属于map中没有数据的类型 (MASK2)
2. DATA段的内容属于有数据的类型   (MASK)

根据LD指令的设计，当src_reg为2时:

1. 指令的立即数类型为map fd
2. 需要加载到寄存器的值类型为data address
3. 加载到寄存器的值的计算表达式为: `dst = map_val(map_by_fd(imm)) + next_imm`
   1. 首先通过imm(map fd)找到map的地址
   2. 找到map的第一个数据的地址
   3. 加上next_imm,这里next_imm应该就是偏移量了，代表查找第几个数据

当src_reg为1时：

1. 指令的立即数类型为map fd
2. 需要加载到寄存器的值类型为map的地址
3. 加载到寄存器的值的计算表达式为： `dst = map_by_fd(imm)`
   1. 通过imm(map fd)找到map的地址

上文提到我们这段程序无法在内核中运行，其提示 错误only read from bpf_array is supported，这里推断就是在访问bss段的MASK2变量时出现问题，因为从Aya的处理来看，这里将其处理成了读写map本身，而不是map内的数据。这显然是错误的。

在内核中，打印同样错误的测试位于https://github.com/torvalds/linux/blob/master/tools/testing/selftests/bpf/progs/verifier_map_ptr.c 中。

到现在为止，我们就搞明白了用户库和内核是如何协同作用，处理这些全局变量的了。=》 通过map处理一切。



### Aya库的进一步探索

上文对全局变量进行了分析，引入了eBPF对map的支持的介绍，以及用户库和内核应该如何处理，但分析时留下了一个疑问：

```rust
instructions[ins_index].set_src_reg(BPF_PSEUDO_MAP_VALUE as u8);
instructions[ins_index + 1].imm = instructions[ins_index].imm + sym.address as i32;
```

这里设置宽指令第二部分指令的立即数的时候，所作的运算代表什么含义，虽然从结果上看我们知道这应该是变量在map中value的偏移量，但到指令层面，我们还是不太确定。因此我们可以编写一个简单的程序来做判断 ：

```rust
static mut MASKS: [usize;2] = [1,1];
#[kprobe]
pub fn myapp(ctx: ProbeContext) -> u32 {
    unsafe {
        MASKS[0] +=1;
        MASKS[1] +=1;
    }
    0
}
```

与上文的程序非常相似，这里我们声明了一个全局数组变量，其应该位于DATA段中，汇编代码如下：

```
0000000000000000 <myapp>:
       0:       18 01 00 00 00 00 00 00 00 00 00 00 00 00 00 00 r1 = 0 ll
                0000000000000000:  R_BPF_64_64  .data
       2:       79 12 00 00 00 00 00 00 r2 = *(u64 *)(r1 + 0)
       3:       07 02 00 00 01 00 00 00 r2 += 1
       4:       7b 21 00 00 00 00 00 00 *(u64 *)(r1 + 0) = r2
       5:       18 01 00 00 08 00 00 00 00 00 00 00 00 00 00 00 r1 = 8 ll
                0000000000000028:  R_BPF_64_64  .data
       7:       79 12 00 00 00 00 00 00 r2 = *(u64 *)(r1 + 0)
       8:       07 02 00 00 01 00 00 00 r2 += 1
       9:       7b 21 00 00 00 00 00 00 *(u64 *)(r1 + 0) = r2
      10:       b7 00 00 00 00 00 00 00 r0 = 0
      11:       95 00 00 00 00 00 00 00 exit
```

主要观察第0和第5行对应的LD指令，可以看到第5行与第0行的差别在于其立即数部分有一个08。

我们再来看一下Aya库的处理过程：

```
relocating map by section index 5, kind Data at insn 0 in section 3
relocate_maps: map data is not empty, set src_reg to BPF_PSEUDO_MAP_VALUE: 2
relocate_maps: set next imm to sym.address:0 + ins.imm:0 = 0
relocate_maps: set imm to fd: 11

relocating map by section index 5, kind Data at insn 5 in section 3
relocate_maps: map data is not empty, set src_reg to BPF_PSEUDO_MAP_VALUE: 2
relocate_maps: set next imm to sym.address:0 + ins.imm:8 = 8
relocate_maps: set imm to fd: 11
```

与上文的处理过程非常相似，不过这里我们就可以看到对变量在map的value 中的偏移的计算过程了。

#### 更新map

我们观察到Aya为这些全局变量创建了map，那为什么eBPF程序在访问这些变量的时候，可以得到正确的初始化值呢。

通过阅读Aya的源代码，可以发现，创建完map后，Aya并没有就直接开始处理指令的重定位过程，在原来的执行流程上，还有一个对map更新的过程：

```
Bpf::load
->EbpfLoader.load
	->MapData::create // 创建映射
	->map.finalize    // map数据更新
	->obj.relocate_maps
		->relocate_maps // 修改指令
```

在`map.finalize` 函数中，其实现如下:

```rust
if !obj.data().is_empty() && obj.section_kind() != EbpfSectionKind::Bss {
            log::error!("map data is not empty, but section kind is not BSS, {:?}", obj.section_kind());
            let data = obj.data();
            let value = u64::from_le_bytes(data[0..8].try_into().unwrap());
            log::error!("bpf_map_update_elem_ptr, key_ptr: {:?}, value_ptr: {:?}, value: {}",&0 as *const _, obj.data_mut().as_mut_ptr(),value);
            bpf_map_update_elem_ptr(fd.as_fd(), &0 as *const _, obj.data_mut().as_mut_ptr(), 0)
                .map_err(|(_, io_error)| SyscallError {
                    call: "bpf_map_update_elem",
                    io_error,
                })
                .map_err(MapError::from)?;
        }
        if obj.section_kind() == EbpfSectionKind::Rodata {
            bpf_map_freeze(fd.as_fd())
                .map_err(|(_, io_error)| SyscallError {
                    call: "bpf_map_freeze",
                    io_error,
                })
                .map_err(MapError::from)?;
        }
```

对于全局变量来说，其数据会被放在ELF的.rodata/data/bss段中，而这些段在创建对应的Map的时候，其Map类型为BPF_MAP_TYPE_ARRAY ，我们会在下文中介绍这个类型。这里理解为这个Map只有一个key，其值应该是这些段中的内容。内核在创建这个类型的Map时，会对其进行0初始化。所以这里的处理是判断这些段中是否有数据，如果有的话，就调用系统调用来将这些数据更新到Map的value当中。这就解释了为什么eBPF程序在读取变量的时候可以获取正确的初始值。





## eBPF ELF

https://www.ietf.org/archive/id/draft-thaler-bpf-elf-00.html#section-4.1-6.8 

关于引入BPF_MAP_TYPE_ARRAY 类型的原因和相关的提交:

https://lore.kernel.org/lkml/1415069656-14138-4-git-send-email-ast@plumgrid.com/#Z31include:uapi:linux:bpf.h 添加了BPF_MAP_TYPE_ARRAY 的初步支持

https://lore.kernel.org/bpf/20190228231829.11993-7-daniel@iogearbox.net/t/ 后续的一系列提交

https://github.com/torvalds/linux/commit/d8eca5bbb2be9 bpf: implement lookup-free direct value access for maps

上文对Aya这样的用户态eBPF处理库做了分析，明确了这些库做的事情。我们就可以仿照其实现，实现一个处理库，简化而言，我们的库包含的功能如下:

1. 首先这个库不应该依赖操作系统的功能，我们将对操作系统的依赖抽象出来，以方便其移植到任何一个os上，这可以通过trait系统实现
2. 库应该包含一个公共部分，一个内核使用的部分，一个用户态使用的部分，公共部分定义的数据结构需要被另外两个部分使用

用户态使用的部分包含的功能与Aya的相同：

1. 处理编译出来的eBPF程序，调用系统调用创建Map，获得对应的文件描述符
2. 根据需要，更新Map的值
3. 根据重定位信息，对eBPF程序的相关指令做修改
4. 加载eBPF程序到内核中

用户态对系统调用的依赖通过trait抽象出来。

内核态使用的部分包含的功能与linux相同：

1. 创建和管理Map
2. 在eBPF执行前，对程序再次进行修改，这部分修改主要是将对Map的访问读写修改为对数据的具体指针
3. 执行程序，获得结果

现阶段我们实现的比较简单，主要是参考Aya的处理过程。把整个流程打通。在后续设计中，我们可以更多参考Aya的实现过程。

现在我们的实现可以执行下面的eBPF程序：

```c
#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>

int counter = 0;
int counter2 = 1;
SEC("xdp")
int hello(void *ctx) {
    counter++;
    counter2++;
    return counter2 + counter;
}

char LICENSE[] SEC("license") = "Dual BSD/GPL";
```

具体的实现和测试见https://github.com/os-module/eebpf/tree/first_test





## windows 的ebpf移植方案

https://github.com/microsoft/ebpf-for-windows?tab=readme-ov-file

![架构概述](./assert/ArchitectureDiagram.png)

[PREVAIL](https://github.com/vbpf/ebpf-verifier?tab=readme-ov-file)  ebpf验证器，windows将其用在用户态对ebpf程序进行检查

[ubpf](https://github.com/iovisor/ubpf) 使用ubpf进行JIT编译，并在内核态执行





## reference

https://docs.kernel.org/bpf/ linux kernel对ebpf的介绍，很多信息位于这里

https://docs.cilium.io/en/latest/bpf/ xdp中对ebpf的介绍部分也很详细

https://terenceli.github.io/%E6%8A%80%E6%9C%AF/2020/08/09/ebpf-with-tracepoint tracepoint如何和ebpf程序结合

https://kinvolk.io/blog/2018/10/exploring-bpf-elf-loaders-at-the-bpf-hackfest/   BPF ELF 加载器，介绍了BPF如何被处理的，与我们对eBPF全局变量的分析相关