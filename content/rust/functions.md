# 函数与表达式

> 对应知识点：k1-1-4 · 预计 25 分钟 · 难度 ★☆☆

## 学习目标

- 写出类型完整的函数签名
- 用「表达式 vs 语句」解释 Rust 的返回值机制

## 一、函数签名：类型是契约

```rust
fn add(a: i32, b: i32) -> i32 {
    a + b              // 块尾表达式：无分号 = 返回值
}
```

- **参数必须标注类型**（返回值也标；单表达式可省略返回类型注解但建议不省）
- 命名惯例：snake_case（编译器对不符者给警告）

## 二、表达式 vs 语句（Rust 的分水岭）

- **表达式**（expression）求值：`5`、`x + 1`、`if c { 1 } else { 2 }`、`{ let a = 1; a + 2 }`
- **语句**（statement）不产生值：`let x = 5;`（let 是语句）、函数调用语句、加过分号的表达式

```rust
fn f() -> i32 {
    5      // ✅ 表达式，作为返回值
}
fn g() -> i32 {
    5;     // ❌ 加了分号变成语句，块返回 ()，与 i32 不符（E0308）
}
```

报错信息会精确告诉你「expected i32, found ()」——这是理解 Rust 类型化块的关键一课。

`return` 用于**提前返回**：

```rust
fn classify(n: i32) -> &'static str {
    if n < 0 {
        return "negative";   // 提前返回
    }
    "non-negative"           // 尾表达式
}
```

## 三、块也是表达式

```rust
let y = {
    let a = 3;
    let b = 4;
    a * b          // y = 12
};
```

这一机制是后面 `match`、`if let`、`loop { break v }` 一切「表达式化控制流」的基石：**Rust 里几乎没有「不能求值」的控制结构**。

## 四、动手实验

1. 实现 `fn bmi(weight_kg: f64, height_m: f64) -> f64`，注意不要除零（返回什么？先想想，再对照 k1-6-2）
2. 写一个块表达式求 1..=100 之和（配合 for 或迭代器）
3. 故意在尾表达式加分号，抄录 E0308 报错原文

## 常见误区

- 用 `return` 包住所有返回 → 尾表达式才是 Rust 惯用风格
- 以为 `let` 也是表达式 → 它是语句，`let a = (let b = 1;)` 不成立
- 忽略命名警告 → 警告即债务，见 k0-3-2

## 验证清单

- [ ] 能解释「分号把表达式变成语句」
- [ ] 复现过 E0308
- [ ] 写过至少一个块表达式赋值

## 延伸阅读

- The Rust Book 3.3 — https://doc.rust-lang.org/book/ch03-03-how-functions-work.html
- Rust Reference · Expressions — https://doc.rust-lang.org/reference/expressions.html
