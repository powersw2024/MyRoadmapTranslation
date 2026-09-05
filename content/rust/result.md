# Result 与可恢复错误

> 对应知识点：k1-6-2 · 预计 30 分钟 · 难度 ★★☆

## 学习目标

- 用 match 完整处理 Result
- 为不同场景选择 unwrap/expect/unwrap_or/?

## 一、Result 类型

```rust
enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

与 Option 的区别：Err **携带失败原因**。凡是「可能失败且失败有原因」的操作都返回它：文件 IO、解析、网络。

```rust
use std::fs::File;

let f = File::open("config.toml");
match f {
    Ok(file) => println!("opened {file:?}"),
    Err(e) => eprintln!("failed: {e}"),   // e 实现了 Display
}
```

## 二、处理策略谱系

```rust
let f = File::open("cfg");

f.unwrap();              // Err 即 panic：原型/测试
f.expect("配置文件必须存在"); // panic 带原因：不可恢复约定
f.unwrap_or_default();   // 失败给默认值
let _ = f;               // 忽略（几乎总是错误做法）
f?;                      // 向上层传播（k1-6-3，生产最常用）
```

决策口诀：**业务上可恢复 → Result 传播；程序 bug/约定破坏 → panic（或断言）**。

## 三、main 返回 Result

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string("config.toml")?; // ? 在 main 也能用
    println!("{content}");
    Ok(())
}
```

`Box<dyn Error>` 是「万能错误」入口，小型工具程序的最佳默认。

## 四、动手实验

写 `fn read_config(path: &str) -> Result<String, std::io::Error>`：用 match 打印「存在/不存在」两种路径的结果；再把 main 改为返回 Result 并用 ? 链接三步：读文件→按行解析→打印首行。

## 常见误区

- 在库代码里 unwrap → 剥夺调用者的恢复权
- 混淆 Option 与 Result 的场景：可能没有值 → Option；操作可能失败且有原因 → Result
- 忽略 `#[must_use]` 警告 → 不处理 Result 编译器会警告，警告即债务

## 验证清单

- [ ] 用 match 完整处理过一次 File::open
- [ ] main 已返回 Result 并用 ?
- [ ] 说出四种处理策略的适用场景

## 延伸阅读

- The Rust Book 9.2 — https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html
- std::result — https://doc.rust-lang.org/std/result/
