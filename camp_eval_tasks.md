# Camp 评测项详细说明

这份文档基于当前仓库 `main` 分支的 README、评分脚本和练习源码整理，目的不是复述“怎么跑脚本”，而是回答更实际的问题:

- 每个 100 分题到底在考什么
- 通过条件是什么
- 你真正需要补哪一层能力
- 建议先看哪些文件

总入口是 [scripts/total-test.sh](./scripts/total-test.sh)。它顺序执行 6 个脚本，每项 100 分，总分 600。

## 总览

| 题目 | 评分脚本 | 主要考点 |
| --- | --- | --- |
| `print_with_color` | `scripts/test-print.sh` | 控制台彩色输出 |
| `ramfs_rename` | `scripts/test-ramfs_rename.sh` | VFS/ramfs 的同目录重命名 |
| `alt_alloc` | `scripts/test-alt_alloc.sh` | 切换并跑通备用分配器 |
| `support_hashmap` | `scripts/test-support_hashmap.sh` | 让堆分配足以支撑大量小对象分配 |
| `sys_map` | `scripts/test-sys_map.sh` | 最小可用 `mmap` 实现 |
| `simple_hv` | `scripts/test-simple_hv.sh` | 最小 hypervisor/guest 启动退出链路 |

---

## 1. `print_with_color`

### 评分标准

脚本运行 `make run A=exercises/print_with_color/`，检查最近 20 行输出:

- 必须包含 ANSI 颜色转义序列，也就是类似 `\x1b[` 的内容
- 去掉颜色控制符后，必须还能看到 `Hello, Arceos!`

对应脚本: [scripts/test-print.sh](./scripts/test-print.sh)

### 当前练习在做什么

应用源码很简单，只打印:

```rust
println!("[WithColor]: Hello, Arceos!");
```

对应文件: [arceos/exercises/print_with_color/src/main.rs](./arceos/exercises/print_with_color/src/main.rs)

### 这题真正要求你做什么

这题不是在考复杂内核机制，本质是在考“终端输出链路能不能打出带颜色的文本”。

最小可行目标:

- 最终串口/控制台上出现一行带 ANSI 颜色的 `Hello, Arceos!`

通常有两种做法:

1. 最直接
   在应用层手工输出 ANSI 转义序列，例如前景色 + 文本 + reset。

2. 稍微工程化一点
   在 `println!` 或控制台输出路径里识别特定前缀，比如 `[WithColor]`，然后自动插入颜色控制码。

### 建议理解成什么范围

从评分脚本看，这题只关心结果，不关心你是否做了通用彩色日志系统。也就是说:

- 不要求彩色日志框架
- 不要求不同 log level 上不同颜色
- 不要求解析复杂标记语法

能稳定输出一行带颜色的 `Hello, Arceos!` 就够了。

### 优先查看文件

- [arceos/exercises/print_with_color/src/main.rs](./arceos/exercises/print_with_color/src/main.rs)
- [arceos/ulib/axstd/src/macros.rs](./arceos/ulib/axstd/src/macros.rs)
- [arceos/ulib/axstd/src/io/stdio.rs](./arceos/ulib/axstd/src/io/stdio.rs)

---

## 2. `ramfs_rename`

### 评分标准

脚本运行:

```sh
make run A=exercises/ramfs_rename/ BLK=y
```

最后一行必须是:

```text
[Ramfs-Rename]: ok!
```

对应脚本: [scripts/test-ramfs_rename.sh](./scripts/test-ramfs_rename.sh)

### 当前练习在做什么

练习逻辑很清楚:

1. 在 `/tmp/f1` 创建文件并写入 `"hello"`
2. 调用 `fs::rename("/tmp/f1", "/tmp/f2")`
3. 重新打开 `/tmp/f2` 并读回内容
4. 成功后打印 `[Ramfs-Rename]: ok!`

对应文件:

- [arceos/exercises/ramfs_rename/src/main.rs](./arceos/exercises/ramfs_rename/src/main.rs)
- [arceos/exercises/ramfs_rename/src/ramfs.rs](./arceos/exercises/ramfs_rename/src/ramfs.rs)

其中注释已经把范围限定死了:

- 只要求 `rename`
- 不要求 `move`
- 也就是只需要处理“同一目录/同一文件系统内改名”

### 这题真正要求你做什么

这题在考文件系统重命名能力是否从上到下打通:

1. 应用层 `std::fs::rename`
2. `arceos_api::fs::ax_rename`
3. `axfs::api::rename`
4. root VFS 的路径分发
5. 具体文件系统节点的 `rename`

调用链可以从这些文件看出来:

- [arceos/ulib/axstd/src/fs/mod.rs](./arceos/ulib/axstd/src/fs/mod.rs)
- [arceos/api/arceos_api/src/imp/fs.rs](./arceos/api/arceos_api/src/imp/fs.rs)
- [arceos/modules/axfs/src/api/mod.rs](./arceos/modules/axfs/src/api/mod.rs)
- [arceos/modules/axfs/src/root.rs](./arceos/modules/axfs/src/root.rs)

### 通过这题至少需要满足什么

- `/tmp` 挂载的是 ramfs，`rename` 必须能作用在这个挂载点里
- 改名后旧路径失效，新路径可读
- 文件内容不能丢
- 不需要支持跨挂载点移动
- 不需要支持目录树搬迁

### 这题更像在补哪里

重点不是应用本身，而是:

- 根目录挂载分发是否把 `/tmp/...` 正确转给 ramfs
- 具体文件系统是否实现了 `rename`
- `rename` 的语义是否符合“同 fs 改名”

如果你想对照一个已经实现的后端，`fatfs` 有现成的 `rename`:

- [arceos/modules/axfs/src/fs/fatfs.rs](./arceos/modules/axfs/src/fs/fatfs.rs)

它能帮助你理解 VFS 层期望的接口语义。

---

## 3. `alt_alloc`

### 评分标准

脚本运行:

```sh
make A=exercises/alt_alloc/ run
```

最后一行必须包含:

```text
Bump tests run OK!
```

对应脚本: [scripts/test-alt_alloc.sh](./scripts/test-alt_alloc.sh)

### 当前练习在做什么

程序会:

1. 创建容量为 3,000,000 的 `Vec`
2. 依次 `push`
3. `sort`
4. 验证结果有序

对应文件: [arceos/exercises/alt_alloc/src/main.rs](./arceos/exercises/alt_alloc/src/main.rs)

依赖里启用了:

```toml
axstd = { workspace = true, features = ["alt_alloc"], optional = true }
```

对应文件: [arceos/exercises/alt_alloc/Cargo.toml](./arceos/exercises/alt_alloc/Cargo.toml)

### 这题真正要求你做什么

这题不是“自己从零写一个通用分配器接口”，而是:

- 打开 `alt_alloc` 这条 feature 链
- 让运行时在初始化时使用备用分配器
- 让这个分配器能承受大块连续增长的 `Vec` 分配和排序过程

运行时里已经有两套初始化路径:

- 默认 `alloc` 路径使用 `axalloc`
- `alt_alloc` 路径使用 `alt_axalloc`

对应文件: [arceos/modules/axruntime/src/lib.rs](./arceos/modules/axruntime/src/lib.rs)

### 这题本质上在考什么

它更像“针对特定负载切换 allocator 并跑通”:

- 负载特征是大数组、顺序追加、较少碎片
- 脚本里的提示词就是 `Bump tests run OK!`

所以它不是要求你做完善的通用堆，而是在特定 workload 下让备用 allocator 正常工作。

### 最低通过要求

- `Vec::with_capacity(N)` 不能炸
- 后续扩容和排序不能触发分配器错误
- 释放路径至少不能破坏程序退出

### 优先查看文件

- [arceos/exercises/alt_alloc/src/main.rs](./arceos/exercises/alt_alloc/src/main.rs)
- [arceos/exercises/alt_alloc/Cargo.toml](./arceos/exercises/alt_alloc/Cargo.toml)
- [arceos/modules/axruntime/src/lib.rs](./arceos/modules/axruntime/src/lib.rs)
- [arceos/api/axfeat/Cargo.toml](./arceos/api/axfeat/Cargo.toml)

---

## 4. `support_hashmap`

### 评分标准

脚本运行:

```sh
make run A=exercises/support_hashmap/
```

最后一行必须包含:

```text
Memory tests run OK!
```

对应脚本: [scripts/test-support_hashmap.sh](./scripts/test-support_hashmap.sh)

### 当前练习在做什么

程序会:

1. 创建 `HashMap<String, u32>`
2. 插入 50,000 个键值对
3. 每个 key 形如 `key_<value>`
4. 遍历 map，解析字符串并校验 value 对应正确

对应文件: [arceos/exercises/support_hashmap/src/main.rs](./arceos/exercises/support_hashmap/src/main.rs)

### 这题真正要求你做什么

这题表面上是 `HashMap`，实质上是在考“你的内存分配能力是否已经足够支撑标准集合类型”。

`HashMap` 会同时依赖这些能力:

- 堆分配
- `String` 分配和扩容
- 较多小对象分配
- rehash / bucket 扩容时的重新分配

所以这题不是让你实现 `HashMap` 本身，而是让底层环境足够稳定，使标准集合可以直接工作。

### 它和 `alt_alloc` 的区别

- `alt_alloc` 偏向大块、顺序型分配压力
- `support_hashmap` 偏向大量小块对象、字符串、哈希表扩容

这两题一起看，评测者实际上是在验证:

- 你的 allocator 不是只能跑一个简单 `Vec`
- 至少对常见 Rust 容器负载也能工作

### 最低通过要求

- `alloc` 功能链要通
- `String` / `format!` 相关分配不能出错
- 频繁插入 `HashMap` 时不能崩
- 遍历时不能读出错误数据

### 优先查看文件

- [arceos/exercises/support_hashmap/src/main.rs](./arceos/exercises/support_hashmap/src/main.rs)
- [arceos/ulib/axstd/Cargo.toml](./arceos/ulib/axstd/Cargo.toml)
- [arceos/modules/axruntime/src/lib.rs](./arceos/modules/axruntime/src/lib.rs)
- [arceos/modules/axalloc/src/lib.rs](./arceos/modules/axalloc/src/lib.rs)

---

## 5. `sys_map`

### 评分标准

脚本会先:

1. `make payload`
2. 把 `payload/mapfile_c/mapfile` 塞进磁盘镜像
3. 运行 `make run A=exercises/sys_map/ BLK=y`

输出中必须出现:

```text
Read back content: hello, arceos!
```

对应脚本: [scripts/test-sys_map.sh](./scripts/test-sys_map.sh)

### 当前练习在做什么

这个练习本身是一个最小“单体内核 + 用户程序加载器”:

1. 创建用户地址空间
2. 把 `/sbin/mapfile` ELF 装进用户地址空间
3. 建立用户栈
4. 进入用户态执行
5. 在 trap handler 里处理若干 syscall

对应文件:

- [arceos/exercises/sys_map/src/main.rs](./arceos/exercises/sys_map/src/main.rs)
- [arceos/exercises/sys_map/src/loader.rs](./arceos/exercises/sys_map/src/loader.rs)
- [arceos/exercises/sys_map/src/task.rs](./arceos/exercises/sys_map/src/task.rs)
- [arceos/exercises/sys_map/src/syscall.rs](./arceos/exercises/sys_map/src/syscall.rs)

其中最关键的信号是:

```rust
fn sys_mmap(...) -> isize {
    unimplemented!("no sys_mmap!");
}
```

所以这题的核心工作非常明确: 实现 `sys_mmap`。

### 用户态 payload 在做什么

被加载的 C 程序会:

1. 新建一个文件 `test_file`
2. 向里面写入 `hello, arceos!`
3. 重新只读打开
4. 调用 `mmap(NULL, 32, PROT_READ, MAP_PRIVATE, fd, 0)`
5. 直接把返回地址当字符串打印

对应文件: [arceos/payload/mapfile_c/mapfile.c](./arceos/payload/mapfile_c/mapfile.c)

这已经把题目范围压得很小:

- 只需要支持文件映射
- 只需要支持读映射
- 只需要支持 `MAP_PRIVATE`
- 长度只有 32 字节
- 偏移是 0

### 这题真正要求你做什么

实现一个“够用就行”的 `mmap`:

1. 从 syscall 参数里解析:
   - `addr`
   - `length`
   - `prot`
   - `flags`
   - `fd`
   - `offset`

2. 把 `prot` 转成页表权限
   代码里已经给了 `MmapProt -> MappingFlags` 的转换。

3. 从当前任务拿到用户地址空间
   `TaskExt` 里已经保存了 `aspace`。

4. 为映射选择一段用户虚拟地址
   如果不支持 `MAP_FIXED`，可以先只处理 `addr == NULL` 的情况。

5. 在用户地址空间上 `map_alloc`

6. 从 `fd` 对应文件读出内容，拷贝到映射页

7. 返回用户可见的映射起始地址

### 最低通过实现可以多简化

为了通过这个仓库的测试，通常可以只支持下面这一小撮语义:

- `addr == NULL`
- `offset == 0`
- `MAP_PRIVATE`
- `PROT_READ`
- 文件映射，不做匿名映射
- 只映射一页或按页向上取整
- 不实现 `munmap`、`mprotect`、`mremap`

换句话说，这不是完整 Linux `mmap`，而是“让这个 payload 正常跑完”的最小版本。

### 这题考的是哪一层

它在考一个最小内核是否具备:

- 用户态 ELF 加载
- 用户页表映射
- trap/syscall 分发
- 文件系统读接口
- 文件到内存的映射能力

### 优先查看文件

- [arceos/exercises/sys_map/src/syscall.rs](./arceos/exercises/sys_map/src/syscall.rs)
- [arceos/exercises/sys_map/src/task.rs](./arceos/exercises/sys_map/src/task.rs)
- [arceos/exercises/sys_map/src/loader.rs](./arceos/exercises/sys_map/src/loader.rs)
- [arceos/payload/mapfile_c/mapfile.c](./arceos/payload/mapfile_c/mapfile.c)

---

## 6. `simple_hv`

### 评分标准

脚本会先:

1. `make payload`
2. 把 `payload/skernel2/skernel2` 放进磁盘镜像
3. 运行 `make run A=exercises/simple_hv/ BLK=y`

输出中必须出现:

```text
Shutdown vm normally!
```

对应脚本: [scripts/test-simple_hv.sh](./scripts/test-simple_hv.sh)

### 当前练习在做什么

这题已经给了一个最小 hypervisor 骨架:

1. 新建一份 guest 地址空间
2. 从 `/sbin/skernel2` 读取 guest binary
3. 把 guest 映射到固定地址 `0x8020_0000`
4. 准备 guest 寄存器上下文
5. 写入 `hgatp`，建立二阶段地址翻译
6. 通过 `_run_guest` 进入 guest
7. guest 退出时，在宿主里解析 trap 原因

关键文件:

- [arceos/exercises/simple_hv/src/main.rs](./arceos/exercises/simple_hv/src/main.rs)
- [arceos/exercises/simple_hv/src/loader.rs](./arceos/exercises/simple_hv/src/loader.rs)
- [arceos/exercises/simple_hv/src/vcpu.rs](./arceos/exercises/simple_hv/src/vcpu.rs)
- [arceos/exercises/simple_hv/src/guest.S](./arceos/exercises/simple_hv/src/guest.S)
- [arceos/exercises/simple_hv/src/csrs.rs](./arceos/exercises/simple_hv/src/csrs.rs)

### guest 程序在做什么

`skernel2` 的入口非常短:

- 读 `mhartid` 到 `a1`
- 从固定物理地址取值到 `a0`
- 设置 `a7 = 8`
- 执行 `ecall`

对应文件: [arceos/payload/skernel2/src/main.rs](./arceos/payload/skernel2/src/main.rs)

宿主侧在 `vmexit_handler` 中把这个 `ecall` 当作 `VirtualSupervisorEnvCall` 处理，并且检查:

- `a0 == 0x6688`
- `a1 == 0x1234`

满足后打印:

```text
Shutdown vm normally!
```

### 这题真正要求你做什么

这题要求你把一个最小的 guest 生命周期跑通:

1. guest 镜像装载
   把 `/sbin/skernel2` 内容拷贝到 guest 内存。

2. guest 页表/二阶段映射
   `prepare_vm_pgtable()` 里写 `hgatp`，并做 `hfence_gvma_all()`。

3. guest 初始上下文
   `prepare_guest_context()` 里设置:
   - `hstatus`
   - guest `sstatus`
   - guest `sepc`

4. 进入 guest
   `_run_guest` 负责保存/恢复 host 与 guest 的寄存器和 CSR。

5. 处理 VM exit
   至少要识别 `VirtualSupervisorEnvCall`，把 SBI 消息解出来。

6. 识别关机路径
   遇到 reset/shutdown 类 SBI 请求时，取出 guest `a0/a1`，验证值正确并退出。

### 这题不是在要求什么

它不是一个完整虚拟机监控器，不要求:

- 多虚机
- 设备虚拟化
- 完整 SBI 覆盖
- 中断注入框架
- 复杂 guest 内存管理

它只要求“能把一个极小 guest 跑起来，然后正常退出来”。

### 最低通过实现的重点

- guest 二进制确实被放到了 `VM_ENTRY`
- `hstatus` 和 `sepc` 要正确
- `hgatp` 要正确指向 guest 页表根
- guest `ecall` 要能回到宿主
- 宿主能从保存的 guest 寄存器里拿到 `a0/a1`

### 额外提示

这个练习和 `tour/h_1_0` 高度相似，后者几乎就是一个参考版本。卡住时可以对照:

- [arceos/tour/h_1_0/src/main.rs](./arceos/tour/h_1_0/src/main.rs)
- [arceos/tour/h_1_0/src/vcpu.rs](./arceos/tour/h_1_0/src/vcpu.rs)
- [arceos/tour/h_1_0/src/guest.S](./arceos/tour/h_1_0/src/guest.S)

---

## 推荐的做题顺序

如果目的是尽快把总评测跑通，建议顺序是:

1. `print_with_color`
   最独立，改动最小。

2. `ramfs_rename`
   主要是 VFS/ramfs 能力打通。

3. `alt_alloc`
   验证备用 allocator feature 链。

4. `support_hashmap`
   用来确认 allocator 不是只对单一 workload 有效。

5. `sys_map`
   第一个真正需要你补“最小内核机制”的题。

6. `simple_hv`
   最复杂，涉及 guest 上下文、CSR 和 VM exit。

## 一句话总结

这 6 个评测项不是平均分布在各个主题上，而是非常集中地考三类能力:

1. 基础接口打通
   `print_with_color`、`ramfs_rename`

2. 内存分配可靠性
   `alt_alloc`、`support_hashmap`

3. 最小内核机制
   `sys_map`、`simple_hv`

如果你后面要真正开始做题，最值得优先精读的不是 `README`，而是各题自己的 `src/main.rs` 和对应的评分脚本；这两个文件合起来，已经把“只需要做多小的功能”说得非常具体了。
