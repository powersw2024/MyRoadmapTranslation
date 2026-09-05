# 变量、可变性与遮蔽

> 对应知识点：k1-1-2 · 预计 25 分钟 · 难度 ★☆☆

## 学习目标

- 解释 Rust 为什么默认不可变
- 区分 `mut` 修改与遮蔽（shadowing）重建
- 说出 const 与 let 的三点差异

## 一、默认不可变

```rust
fn main() {
    let x = 5;
    x = 6; // ❌ E0384: cannot assign twice to immutable variable
}
```

这不是麻烦，是**默认安全**：不可变变量让「这个值从头到尾没变过」成为编译器可验证的事实。并发代码、重构、跨团队阅读都因此受益。需要变化时**显式举手**：

```rust
let mut x = 5;
x = 6; // ✅
```

Rust 的哲学：**显式可变，隐藏即风险**。

## 二、遮蔽（Shadowing）

```rust
let x = 5;
let x = x + 1;      // 新的 x，值为 6
let x = "six";      // 又一个新 x，类型都换了！
println!("{x}");    // six
```

遮蔽创建的是**全新的绑定**，旧变量被遮住（不是销毁，是看不见）。与 `mut` 的本质区别：

| | `let mut x` | `let x`（遮蔽） |
| --- | --- | --- |
| 变量个数 | 1 个 | N 个 |
| 可改类型 | ❌ 类型固定 | ✅ 可换类型 |
| 中间状态可见 | ✅ 旧值还能被引用 | ❌ 遮蔽即失效 |

典型用法：解析管道 `let input = "3"; let input: i32 = input.parse().unwrap();` ——名字保持语义，类型进化，且不会有人误用字符串版本的值。

## 三、const 与 let

```rust
const MAX_POINTS: u32 = 100_000; // 必须标注类型；编译期常量
```

- const **必须**标注类型
- const 是**编译期**求值（可用在数组长度、match 模式等编译期上下文）
- const 可声明在任何作用域（包括全局），且全局绑定默认不可变，**没有全局 mut**（那需要 unsafe，本教程不鼓励）

数字可加下划线提升可读性：`1_000_000`。

## 四、动手实验

```rust
fn main() {
    let x = 5;
    // x = 6;                  // ① 打开注释，读 E0384
    let mut y = 5;
    y = 6;                     // ② 正常
    let z = 5;
    let z = z + 1;             // ③ 遮蔽
    let z = z.to_string();     // ④ 遮蔽换类型
    println!("{x} {y} {z}");
}
```

逐个打开/关闭注释行，观察编译器提示——**让编译器教你语言**是本教程的第一原则。

## 常见误区

- 「遮蔽就是修改变量」→ 不，是创建新绑定，生命周期与引用都不同
- 「const 跟 let mut 一样能存运行时结果」→ 不行，const 必须编译期可知
- 全局可变状态用 `static mut` → 几乎总是错误答案，请用 `Mutex`/架构调整

## 验证清单

- [ ] 复现过 E0384 并修复
- [ ] 写过一次「遮蔽换类型」的解析管道
- [ ] 能口头说出 const 与 let 的三点差异

## 延伸阅读

- The Rust Book 3.1 — https://doc.rust-lang.org/book/ch03-01-variables-and-mutability.html
- 错误索引 E0384 — https://doc.rust-lang.org/error_codes/E0384.html
