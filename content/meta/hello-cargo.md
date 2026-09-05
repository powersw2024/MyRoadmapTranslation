# 第一个程序 hello cargo

> 对应知识点：k0-2-3 · 预计 20 分钟 · 难度 ★☆☆

## 学习目标

- 创建、构建并运行一个 Cargo 项目
- 说清 Cargo.toml 与 target/ 的分工

## 一、创建项目

```bash
cargo new hello_rust
cd hello_rust
```

生成的结构：

```text
hello_rust/
├── Cargo.toml      # 包清单：元数据与依赖（唯一你需要手改的顶层文件）
└── src/
    └── main.rs     # 程序入口
```

`Cargo.toml` 内容：

```toml
[package]
name = "hello_rust"   # 包名
version = "0.1.0"     # 语义化版本
edition = "2021"      # Rust 版本方言（编译器按 edition 解释语法）
```

## 二、main.rs 与运行

`src/main.rs`：

```rust
fn main() {
    println!("Hello, world!");
}
```

把输出改成你的名字，然后：

```bash
cargo run     # 构建 + 运行（开发内循环最高频命令）
cargo build   # 只构建，产物在 target/debug/
```

直接运行二进制：

```bash
./target/debug/hello_rust
```

## 三、理解构建产物

- `target/debug/`：开发构建，带调试符号、包含整数溢出检查，**较慢但更安全**
- `target/release/`：`cargo build --release` 产物，开启优化，用于性能验证与发布

`.gitignore` 中已包含 `/target`——构建产物永不进版本库。

## 四、修改代码的增量编译

Cargo 记录依赖与编译指纹，只重编译变化的部分。第一次 `cargo run` 较慢（全量），之后秒级——这就是为什么日常开发用 `cargo check`（更快，只做类型/借用检查不生成产物）。

## 验证清单

- [ ] `cargo run` 输出了你修改后的文字
- [ ] 能直接执行 `./target/debug/hello_rust`
- [ ] 能说出 Cargo.toml 和 target/ 分别存放什么

## 延伸阅读

- The Rust Book 1.2 Hello, Cargo! — https://doc.rust-lang.org/book/ch01-02-hello-cargo.html
- The Cargo Book · Getting Started — https://doc.rust-lang.org/cargo/getting-started/index.html
