# 所有权三规则

> 对应知识点：k1-3-1 · 预计 35 分钟 · 难度 ★★☆

## 学习目标

- 默写并理解所有权三规则
- 解释 Rust 与 GC 语言/C 在内存管理上的根本差异
- 读懂 E0382 报错

## 一、为什么需要所有权

三种内存管理流派的取舍：

| 流派 | 时机 | 代价 |
| --- | --- | --- |
| 手动（C/C++） | 程序员 malloc/free | 双重释放、悬垂指针、泄漏 |
| GC（Java/Go/JS） | 运行时扫描回收 | 停顿、内存开销、不可预测 |
| **所有权（Rust）** | **编译期插入 drop** | 学习曲线（一次性付清） |

Rust 在**编译期**静态计算每个值的释放点，运行时零 GC、无悬垂——代价是你必须按「单一所有者」的方式组织代码。

## 二、三规则

1. **每个值都有一个所有者**（owner）——持有它的变量
2. **同一时刻只能有一个所有者**——赋值/传参 = 所有权转移（move）
3. **所有者离开作用域，值被 drop**——确定性析构（RAII）

```rust
{
    let s = String::from("hello"); // s 拥有堆上的字符串
    // 使用 s ……
}   // ← s 离开作用域，drop 自动调用，堆内存立即释放
```

`drop` 的执行点是**编译期确定、运行时精确**的：没有 GC 扫描，没有不确定的析构延迟。文件句柄、锁、socket 都靠这一机制自动清理（RAII）。

## 三、栈与堆：为什么 String 有所有权而 i32 没有

- `i32` 等标量固定大小、存栈上，复制成本 = 复制几个字节，所以按位复制（Copy）即可，无所有权问题
- `String` 含**堆指针**（ptr/len/capacity 三元组）：如果复制三元组，两个变量都指向同一块堆内存 → 作用域结束双重释放 → 内存不安全。因此赋值只能 **move**：指针转交，原变量作废

```rust
let s1 = String::from("hi");
let s2 = s1;                 // move：s1 的指针转给 s2
// println!("{s1}");         // ❌ E0382: borrow of moved value: s1
let n1 = 5;
let n2 = n1;                 // Copy：栈上按位复制
println!("{n1} {n2}");       // ✅ 都活着
```

深拷贝的显式出口是 `.clone()`。

## 四、动手实验

```rust
fn take(s: String) { println!("got: {s}"); }

fn main() {
    let s = String::from("hello");
    take(s);
    // println!("{s}");       // ① E0382：所有权已交给函数
    let s2 = String::from("hi");
    take(s2.clone());          // ② 显式深拷贝，s2 仍可用
    println!("{s2}");
}
```

逐个打开注释，抄录报错原文，然后运行 `rustc --explain E0382` 阅读官方解释。

## 常见误区

- 「move 很慢（复制了数据）」→ move 只复制三元组指针，O(1)
- 「离开作用域才 drop，函数中间不能释放」→ 可用 `std::mem::drop(s)` 提前
- 「所有权规则限制太死」→ 下一站「借用」就是为共享访问设计的出口

## 验证清单

- [ ] 合卷默写三规则（对照 k0-1-1 的间隔重复）
- [ ] 复现 E0382 并能用「唯一所有者」解释它
- [ ] 能向他人解释为什么 `i32` 赋值后原变量可用而 `String` 不行

## 延伸阅读

- The Rust Book 4.1 — https://doc.rust-lang.org/book/ch04-01-understanding-ownership.html
- 错误索引 E0382 — https://doc.rust-lang.org/error_codes/E0382.html
- The Rustonomicon · Moves — https://doc.rust-lang.org/nomicon/moves.html
