# cargo 常用命令

> 对应知识点：k0-3-1 · 预计 25 分钟 · 难度 ★☆☆

## 学习目标

- 建立「check → test → clippy → run」的日常开发循环
- 知道每个命令的适用场景与速度差异

## 一、开发内循环（每分钟都在用）

| 命令 | 作用 | 速度 |
| --- | --- | --- |
| `cargo check` | 类型与借用检查，**不生成可执行文件** | ⚡ 最快 |
| `cargo build` | 编译出 target/debug 产物 | 快 |
| `cargo run` | build + 运行 | 中 |
| `cargo test` | 编译并运行测试 | 中 |
| `cargo clippy` | lint 检查（惯用法/常见错误） | 中 |

**肌肉记忆**：改完代码 → `cargo check`（3 秒反馈）→ 写完功能 → `cargo test` → 提交前 → `cargo clippy && cargo fmt --check`。

## 二、性能与发布

```bash
cargo run --release        # 开优化构建并运行（验证性能必须用 release）
cargo build --release      # 发布产物 target/release/
```

debug 与 release 的行为差异不止速度：整型溢出在 debug 下 panic、release 下回绕。性能测试永远用 `--release`。

## 三、测试相关

```bash
cargo test                 # 跑全部测试（单元+集成+文档）
cargo test borrow          # 只跑名字含 borrow 的测试
cargo test -- --nocapture  # 显示测试中的 println 输出
cargo test --doc           # 只跑文档测试
```

## 四、文档与其他

```bash
cargo doc --open           # 生成并打开 API 文档
cargo fmt                  # 格式化
cargo fmt --check          # 只检查（CI 用）
cargo clippy --fix         # 自动修复部分 lint
cargo tree                 # 打印依赖树
cargo add serde -F derive  # 添加依赖（带 feature）
```

## 验证清单

- [ ] 在一个项目里依次运行过 check/build/test/clippy/doc 并记录各自耗时
- [ ] 能解释 check 与 build 的区别
- [ ] 用 `cargo test <过滤词>` 单独运行过一个测试

## 延伸阅读

- The Cargo Book · Commands — https://doc.rust-lang.org/cargo/commands/index.html
- rustlings（把命令循环变成肌肉记忆）— https://github.com/rust-lang/rustlings
