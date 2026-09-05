# 基本数据类型

> 对应知识点：k1-1-3 · 预计 30 分钟 · 难度 ★☆☆

## 学习目标

- 选对整数类型并说清溢出行为
- 理解 char 是 Unicode 标量值（4 字节）
- 掌握元组与数组的适用场景

## 一、标量类型

**整数**：`i8 i16 i32 i64 i128 isize` 与无符号 `u8 ... usize`，默认推断为 `i32`。`usize` 用于索引与容量（与指针同宽）。

```rust
let a: i64 = -9_223_372_036_854_775_808;
let b = 255u8;          // 后缀标注
let c = 0xff;           // 十六进制
let d = 0b1010_1010;    // 二进制，下划线分组
```

**溢出行为**（重要差异！）：

```rust
let x: u8 = 255;
// debug 构建: x + 1 → panic（attempt to add with overflow）
// release 构建: x + 1 → 0（回绕 wrapping）
```

需要显式语义时用方法族：`checked_add`（返回 Option）、`wrapping_add`（明确回绕）、`saturating_add`（钳制到边界）。

**浮点**：`f32`/`f64`（默认），IEEE-754，永远不要用 `==` 比较浮点，用 `(a - b).abs() < EPS`。

**bool**：1 字节，只有 `true/false`——**不存在真值转换**，`if 5 {}` 直接编译错误。

**char**：4 字节 Unicode 标量值，不是 C 的 1 字节：

```rust
let c = '中';
println!("{}", std::mem::size_of_val(&c)); // 4
```

## 二、复合类型

**元组**：固定长度、元素类型可异、按位置访问，适合临时组合：

```rust
let tup: (i32, f64, char) = (500, 6.4, 'z');
let (x, y, z) = tup;           // 解构
let first = tup.0;             // 点访问
```

函数返回多值、`Result<T, E>` 都是元组思维的产物。

**数组**：**长度编译期固定**、元素同型、在栈上：

```rust
let arr: [i32; 5] = [1, 2, 3, 4, 5];
let zeros = [0; 5];            // 5 个 0
let s = &arr[1..3];            // 切片 &[i32]
```

运行期可变长度请用 `Vec`（见 k1-5-1）。

## 三、动手实验

```rust
fn main() {
    println!("{}", u8::MAX);                       // 255
    println!("{}", i32::MIN);                      // -2147483648
    println!("{}", std::mem::size_of::<char>());   // 4
    let x: u8 = 255;
    // println!("{}", x + 1);      // ① debug 下 panic；cargo build --release 后变成 0
    println!("{}", x.checked_add(1).unwrap_or(0)); // ② 安全接口
}
```

## 常见误区

- 用 `f32` 做钱的计算 → 用整数分单位或定点库
- 把 `char` 当 1 字节 → 它是 4 字节 Unicode 标量；字符串见 k1-5-2
- 依赖 release 的回绕行为 → 那是实现细节，显式用 wrapping/checked 才是契约

## 验证清单

- [ ] 触发过 debug 溢出 panic 并观察 release 差异
- [ ] 用 checked/wrapping 改写过一次算术
- [ ] 用元组解构接收过函数的多返回值

## 延伸阅读

- The Rust Book 3.2 — https://doc.rust-lang.org/book/ch03-02-data-types.html
- std::primitive 文档（每个数值类型的完整方法表）— https://doc.rust-lang.org/std/primitive.i32.html
