# `ramfs_rename` 实现记录

这份文档记录这次把 `ramfs_rename` 题做通的完整过程，包括：

- 问题定位
- 为什么第一次修改后仍然失败
- 最终改了哪些文件
- 每个改动解决了什么问题
- 我实际如何验证

## 1. 题目目标

评分脚本 [scripts/test-ramfs_rename.sh](./scripts/test-ramfs_rename.sh) 的判定很简单：

1. 运行 `make run A=exercises/ramfs_rename/ BLK=y`
2. 检查最后一行是否为：

```text
[Ramfs-Rename]: ok!
```

练习程序本身在 [arceos/exercises/ramfs_rename/src/main.rs](./arceos/exercises/ramfs_rename/src/main.rs) 中做了 4 件事：

1. 创建 `/tmp/f1`
2. 往里面写入 `hello`
3. 调用 `fs::rename("/tmp/f1", "/tmp/f2")`
4. 重新读取 `/tmp/f2`

所以最低目标可以明确成一句话：

`/tmp` 上的 ramfs 必须支持同目录文件重命名。

## 2. 第一轮定位

我先沿着 `rename` 调用链往下看：

1. `std::fs::rename`
   文件：[arceos/ulib/axstd/src/fs/mod.rs](./arceos/ulib/axstd/src/fs/mod.rs)
2. `arceos_api::fs::ax_rename`
   文件：[arceos/api/arceos_api/src/imp/fs.rs](./arceos/api/arceos_api/src/imp/fs.rs)
3. `axfs::api::rename`
   文件：[arceos/modules/axfs/src/api/mod.rs](./arceos/modules/axfs/src/api/mod.rs)
4. `axfs::root::rename`
   文件：[arceos/modules/axfs/src/root.rs](./arceos/modules/axfs/src/root.rs)
5. 具体文件系统节点的 `rename`

然后我读了 ramfs 的实现：

- [arceos/axfs_ramfs/src/lib.rs](./arceos/axfs_ramfs/src/lib.rs)
- [arceos/axfs_ramfs/src/dir.rs](./arceos/axfs_ramfs/src/dir.rs)
- [arceos/axfs_ramfs/src/file.rs](./arceos/axfs_ramfs/src/file.rs)

这里马上能看到第一个缺口：

- `DirNode` 实现了 `lookup/create/remove/read_dir`
- 但没有实现 `rename`

而 `axfs_vfs::VfsNodeOps` 对 `rename` 的默认实现是直接返回 `Unsupported`：

- [./.cargo/registry/src/index.crates.io-6f17d22bba15001f/axfs_vfs-0.1.2/src/lib.rs](/run/media/user0/Mr-why/workspace/opencamp/2026s/2026s-arceos-silicalet-2/.cargo/registry/src/index.crates.io-6f17d22bba15001f/axfs_vfs-0.1.2/src/lib.rs)

所以第一结论很直接：

- ramfs 侧必须新增 `rename`

## 3. 第二个缺口：root VFS 传目标路径的方式不对

继续读 [arceos/modules/axfs/src/root.rs](./arceos/modules/axfs/src/root.rs) 后，我发现第二个问题比第一个更隐蔽。

原始的 `RootDirectory::rename()` 大致是这个意思：

```rust
self.lookup_mounted_fs(src_path, |fs, rest_path| {
    fs.root_dir().rename(rest_path, dst_path)
})
```

这会导致一个关键问题：

- 源路径是按挂载点裁剪后的相对路径
- 目标路径却还是原始全路径

举例：

- 源：`/tmp/f1`
- 目标：`/tmp/f2`

进入挂载点 `/tmp` 后，真正传给底层 fs 的会变成：

- `src_path = "/f1"`
- `dst_path = "/tmp/f2"`

这对 ramfs 来说是不合理的，因为它自己的根目录并不知道外部挂载点名字叫 `/tmp`。

所以第二结论是：

- root VFS 在做 `rename` 时，必须把源路径和目标路径都解析到同一个挂载文件系统里的相对路径

## 4. 第一版实现后为什么还是失败

我先在本地 `arceos/axfs_ramfs` 里补了 `rename`，然后跑：

```sh
./scripts/test-ramfs_rename.sh
```

结果还是失败，而且错误仍然是 `Unsupported`。

这时我继续检查依赖解析，发现了第三个问题：

- 仓库里虽然有本地目录 `arceos/axfs_ramfs`
- 但 `modules/axfs` 和 `exercises/ramfs_rename` 的 `Cargo.toml` 实际依赖写的是 `version = "0.1"`
- Cargo 最终拉到的是 crates.io 上的 `axfs_ramfs`
- 我改的本地 `arceos/axfs_ramfs` 根本没有被练习链接进去

确认方式：

```sh
cargo tree -p ramfs_rename -i axfs_ramfs
```

在修正前，`axfs_ramfs` 不显示本地路径；修正后则显示：

```text
axfs_ramfs v0.1.1 (/.../arceos/axfs_ramfs)
```

所以第三结论是：

- 必须把 `axfs_ramfs` 相关依赖显式切到本地 `path`

## 5. 最终改动

### 5.1 在 `axfs_ramfs` 中实现 `rename`

修改文件：

- [arceos/axfs_ramfs/src/dir.rs](./arceos/axfs_ramfs/src/dir.rs)

我加了三部分逻辑。

#### 1. `rename_node()`

这是最底层的同目录改名操作，逻辑是：

1. 从 `children` 里拿出源节点
2. 如果目标已存在：
   - 如果目标是非空目录，返回 `DirectoryNotEmpty`
   - 否则允许覆盖
3. 把源节点重新插入到新名字下

这一步只负责目录项表层面的“换名字”。

#### 2. `split_parent_path()`

把规范化后的路径拆成：

- 父目录路径
- 最后一级名字

例如：

- `"/f1"` -> `("/", "f1")`
- `"tmp/f1"` -> `("tmp", "f1")`

#### 3. `DirNode::rename()`

这里做路径级别的判断：

1. 先对 `src_path` 和 `dst_path` 做 canonicalize
2. 拆出父目录和文件名
3. 拒绝空名字、`.`、`..`
4. 如果源父目录和目标父目录不同，返回 `Unsupported`
5. 如果父目录是当前目录，直接 `rename_node`
6. 否则先 `lookup()` 找到父目录，再对那个父目录调用 `rename_node`

这里我故意只支持“同一父目录重命名”，原因是这题练习源码已经明确写了：

- `Only support rename, NOT move.`

所以没必要把它扩展成通用跨目录移动。

### 5.2 修正 root VFS 的 `rename` 路由

修改文件：

- [arceos/modules/axfs/src/root.rs](./arceos/modules/axfs/src/root.rs)

我做了两件事。

#### 1. 新增 `mounted_fs_for_path()`

这个辅助函数负责：

- 根据路径找到它真正落在哪个挂载文件系统上
- 返回该 fs 以及该路径在 fs 内部的相对路径

同时我顺手把匹配逻辑改成了“按路径组件匹配”，避免 `/tmp2` 错误命中 `/tmp`。

#### 2. 重写 `root::rename()`

新的流程是：

1. 先把 `old` 和 `new` 都转成绝对规范路径
2. 分别解析出：
   - 源文件所在 fs 和相对路径
   - 目标文件所在 fs 和相对路径
3. 如果任一相对路径为空，拒绝
   - 这能避免对挂载点本身改名
4. 如果源和目标不在同一个 fs 上，拒绝
   - 这题不支持跨 fs move
5. 如果目标已经存在：
   - 是目录则走 `remove_dir`
   - 是文件则走 `remove_file`
6. 最后调用具体 fs 根节点的 `rename(src_rel, dst_rel)`

这样传给 ramfs 的参数就会变成：

- `"/f1"`
- `"/f2"`

而不会再是错误的 `"/tmp/f2"`。

### 5.3 把 `axfs_ramfs` 依赖切到本地路径

修改文件：

- [arceos/modules/axfs/Cargo.toml](./arceos/modules/axfs/Cargo.toml)
- [arceos/exercises/ramfs_rename/Cargo.toml](./arceos/exercises/ramfs_rename/Cargo.toml)

改法很直接：

从：

```toml
axfs_ramfs = { version = "0.1", optional = true }
```

改成：

```toml
axfs_ramfs = { path = "../../axfs_ramfs", optional = true }
```

这是这次实现能真正生效的关键一步。

## 6. 补充测试

### 6.1 `axfs_ramfs` 单元测试

修改文件：

- [arceos/axfs_ramfs/src/tests.rs](./arceos/axfs_ramfs/src/tests.rs)

新增了 `test_rename()`，覆盖：

1. 普通文件改名后旧名不存在、新名能读回原内容
2. 同目录目录名覆盖
3. 跨目录 rename 返回 `Unsupported`

运行结果：

```sh
cargo test -p axfs_ramfs
```

通过。

### 6.2 `axfs` 集成层 `/tmp` rename 测试

修改文件：

- [arceos/modules/axfs/tests/test_common/mod.rs](./arceos/modules/axfs/tests/test_common/mod.rs)

我在原有 `/tmp` 测试流里加了：

1. 写入 `/tmp/dir/test.txt`
2. `rename("/tmp/dir/test.txt", "/tmp/dir/test2.txt")`
3. 确认旧文件 `NotFound`
4. 确认新文件内容仍为 `"test"`

我尝试跑：

```sh
cargo test -p axfs --features myfs test_ramfs -- --nocapture
```

但这条在当前环境里失败了，失败原因不是本次改动，而是已有依赖 `x86_64 0.14.13` 与当前 host 工具链的 trait 签名不兼容，报错点是 `steps_between`。

所以这条集成测试我保留了代码，但没有把它作为最终通过依据。

## 7. 最终验证

最终通过的是题目自己的 QEMU 评分脚本：

```sh
./scripts/test-ramfs_rename.sh
```

关键输出：

```text
Create '/tmp/f1' and write [hello] ...
Rename '/tmp/f1' to '/tmp/f2' ...
Read '/tmp/f2' content: [hello] ok!

[Ramfs-Rename]: ok!
ramfs_rename pass
```

注意脚本前半段仍然会出现一次和 `sudo` / `disk_img` 相关的环境报错，但它没有阻止后续构建、启动 QEMU 和评分判定；这与本次 `rename` 实现无关。

## 8. 这次实现的边界

这次实现是按题目需求收敛过的，不是完整 Unix `rename`：

- 支持：
  - 同一文件系统内
  - 同一父目录下
  - 文件或空目录的重命名/覆盖

- 不支持：
  - 跨文件系统 rename
  - 跨目录 move
  - 更完整的 POSIX `rename` 语义

这是有意为之，因为当前练习只需要：

```text
/tmp/f1 -> /tmp/f2
```

超出这个范围的功能，应该单独作为文件系统增强任务来做，而不应该混进这个训练营单题里。

## 9. 本次改动清单

- [arceos/axfs_ramfs/src/dir.rs](./arceos/axfs_ramfs/src/dir.rs)
- [arceos/axfs_ramfs/src/tests.rs](./arceos/axfs_ramfs/src/tests.rs)
- [arceos/modules/axfs/src/root.rs](./arceos/modules/axfs/src/root.rs)
- [arceos/modules/axfs/tests/test_common/mod.rs](./arceos/modules/axfs/tests/test_common/mod.rs)
- [arceos/modules/axfs/Cargo.toml](./arceos/modules/axfs/Cargo.toml)
- [arceos/exercises/ramfs_rename/Cargo.toml](./arceos/exercises/ramfs_rename/Cargo.toml)

## 10. 一句话总结

这题真正难的不是“写一个 rename 函数”，而是识别出有三个独立问题同时存在：

1. ramfs 本身没实现 `rename`
2. root VFS 转发目标路径的方式不对
3. 实际构建根本没用到本地改过的 `axfs_ramfs`

三个问题全部处理完以后，`ramfs_rename` 才真正通过。
