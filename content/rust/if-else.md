# if/else 与条件表达式

> 对应知识点：k1-2-1 · 预计 15 分钟 · 难度 ★☆☆

## 学习目标

- 掌握 if 作为表达式的赋值用法
- 解释 Rust 为什么没有真值转换与三元运算符

## 一、条件必须是 bool

```rust
let n = 3;
if n != 0 { println!("非零"); }     // ✅

// if n { }        // ❌ E0308: expected `bool`, found integer
// if n == 0 { } else if 1 { }  // ❌ 同上
```

C/JS 里 `if (n)` 的隐式真值转换在 Rust 中被**有意移除**：`n != 0`、`!s.is_empty()` 多敲几个字符，换来读者对条件的确定理解，也消灭了 `=` 误写成赋值的一类 bug。

## 二、if 是表达式

```rust
let n = 7;
let parity = if n % 2 == 0 { "偶" } else { "奇" };
println!("{n} 是{parity}数");
```

三条规则：

1. **分支类型必须一致**（编译器统一推断）：`if c { 1 } else { "一" }` 报 E0308
2. 分支块尾表达式（无分号）即值
3. 可以没有 else（此时整个表达式的类型是 `()`）——用于纯副作用分支

## 三、else if 链与 match 的边界

```rust
let score = 86;
let grade = if score >= 90 {
    'A'
} else if score >= 80 {
    'B'
} else if score >= 60 {
    'C'
} else {
    'D'
};
```

经验边界：

- 条件是**互不相干的布尔判断** → if/else if
- 对**同一个值**按形态分类（枚举、区间、解构）→ match（k1-2-3），穷尽性检查更安全

## 四、Rust 没有 `?:` 三元运算符

`cond ? a : b` 被 `if cond { a } else { b }` 完全覆盖——因为 if 本身就是表达式。语言特性少一个，表达式体系统一一份。

## 动手实验

1. FizzBuzz 单值分支：用 if 表达式给 `result` 赋值并打印（15 的倍数/3/5/其他）
2. 写 `if 5 { }` 抄录 E0308 报错
3. 写一个类型不一致的双分支，读报错中对两侧类型的具体说明

## 常见误区

| 误区 | 事实 |
| --- | --- |
| 条件加括号 `if (x > 0)` | 能编译但 clippy 提示（unused_parens），Rust 惯例不加 |
| 分支尾部分号当"必须的语法" | 加分号会把值变 `()`，赋值场景直接编译失败 |
| 用 if 链对枚举分类 | 可行但失去穷尽性检查，改用 match |

## 验证清单

- [ ] 用 if 表达式完成过一次赋值
- [ ] 复现过"分支类型不一致"的 E0308
- [ ] 能说出 if 链与 match 的适用边界

## 延伸阅读

- The Rust Book 3.5 Control Flow — https://doc.rust-lang.org/book/ch03-05-control-flow.html
- Rust Reference · if expressions — https://doc.rust-lang.org/reference/expressions/if-expr.html
