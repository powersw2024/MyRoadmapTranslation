# String 与 &str

> 对应知识点：k1-5-2 · 预计 30 分钟 · 难度 ★★☆

## 学习目标

- 区分 String（拥有）与 &str（借用切片）
- 解释 len() 返回字节数的原因

## 一、两个类型，两个角色

- `String`：**拥有所有权**的 UTF-8 堆字符串（内部是 `Vec<u8>`），可增长、可修改
- `&str`：字符串**切片视图**，只借用不拥有；字符串字面量 `"hi"` 就是 `&'static str`

```rust
let mut owned = String::from("hello");   // 拥有堆内存
owned.push_str(" world");                 // 可增长
let view: &str = &owned[0..5];            // 借用片段
let lit: &'static str = "literal";        // 编译进二进制
```

## 二、UTF-8：字节与字符

```rust
let s = "中文abc";
println!("{}", s.len());                  // 9：3+3+1+1+1 字节
println!("{}", s.chars().count());        // 5：字符数
// &s[0..2] → panic！字节 2 落在「中」的 UTF-8 序列中间
```

按字节切片可能切进多字节字符内部，Rust 选择 panic 而不是产生非法字符串。按字符/按字节迭代：

```rust
for c in s.chars() { }   // char（Unicode 标量）
for b in s.bytes() { }   // u8
```

## 三、拼接与格式化

```rust
let full = format!("{owned} + {lit}");     // 最常用
owned.push('!'); owned.push_str(" ok");    // 原地追加
let joined = ["a", "b"].join("-");         // -> String
```

## 四、API 设计惯例

函数参数一律 `&str`（调用方传 `&String` 会自动强转）：

```rust
fn greet(name: &str) -> String { format!("hi, {name}") }
let s = String::from("rust");
greet(&s);      // ✅ deref coercion
greet("world"); // ✅ 字面量
```

需要**存储/返回**字符串时才用 `String`（拥有者）。

## 验证清单

- [ ] 复现「按字节切片 panic」并改用 chars 处理
- [ ] 把一个 `&String` 参数重构为 `&str`
- [ ] 说出何时用 String 何时用 &str

## 延伸阅读

- The Rust Book 8.2 — https://doc.rust-lang.org/book/ch08-02-strings.html
