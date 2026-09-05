# rustfmt 与 clippy 入门

> 对应知识点：k0-3-2 · 预计 20 分钟 · 难度 ★☆☆

## 学习目标

- 把 fmt 与 clippy 加入保存即执行的日常流程
- 理解「警告在 CI 中必须变错误」的纪律

## 一、rustfmt：格式不再争论

```bash
cargo fmt            # 格式化整个项目
cargo fmt --check    # 只检查不修改，有未格式化文件则非 0 退出
```

官方默认风格覆盖了空格、换行、链式调用的折行等所有细节。**建议不写或尽量少写 rustfmt.toml**——格式统一的收益来自「所有人用同一套」，配置越多差异越多。

编辑器（VS Code + rust-analyzer）设置保存自动格式化后，这一步是无感的。

## 二、clippy：500+ 条经验之谈

clippy 是官方 lint 工具，捕获「能编译但不好」的代码：

```bash
cargo clippy
```

典型提示：

```rust
// 会提示 clippy::needless_return
fn f() -> i32 { return 5; }

// 会提示 clippy::redundant_clone
let s2 = s.clone(); let s3 = s2.clone(); // 一次多余

// 会提示 clippy::manual_range_contains
if x >= 1 && x <= 10 {}   // 建议 (1..=10).contains(&x)
```

处理策略：

1. **能改就改**：多数 lint 指向更地道、更安全的写法
2. **真误报就豁免并留痕**：

```rust
#[allow(clippy::too_many_arguments)] // 注册回调需要全量参数，见 issue #12
fn register(cb: fn(u8,u8,u8,u8,u8,u8)) {}
```

3. **CI 中一律 `-D warnings`**：

```bash
cargo clippy -- -D warnings
```

警告不升级为错误，就会被无限容忍，三个月后变成 800 条「以后再修」。

## 三、与 CI 的关系

本仓库 `.github/workflows/ci.yml` 中的质量门禁：

```yaml
- run: cargo fmt --check
- run: cargo clippy -- -D warnings
- run: cargo test
```

## 验证清单

- [ ] 体验过一次 clippy 提示并用 --fix 修复
- [ ] 编辑器已配置保存自动格式化
- [ ] 理解 `-D warnings` 为什么必要

## 延伸阅读

- Clippy lint 列表（可按类别浏览）— https://rust-lang.github.io/rust-clippy/master/
- rustfmt Book — https://rust-lang.github.io/rustfmt/
