# 借用与引用

> 对应知识点：k1-3-3 · 预计 30 分钟 · 难度 ★★☆

## 学习目标

- 用引用解决「传值后原变量失效」问题
- 区分 `&T` 与 `&mut T` 的能力边界
- 体会「借用」是 Rust 提供的安全共享出口

## 一、问题：move 太严格了

```rust
fn len(s: String) -> usize { s.chars().count() }

let s = String::from("hello");
let l = len(s);
// println!("{s} {l}");   // ❌ E0382：s 已被 move 进函数
```

每次传参都丢所有权显然不现实。借用（borrow）登场：**传递访问权，不传递所有权**。

```rust
fn len(s: &String) -> usize { s.chars().count() }  // 借用

let s = String::from("hello");
let l = len(&s);
println!("{s} {l}");       // ✅ s 归你，len 只是「借去看了一眼」
```

## 二、两种引用

| 语法 | 名称 | 能力 | 约束 |
| --- | --- | --- | --- |
| `&T` | 不可变借用 | 只读 | 可同时存在任意多个 |
| `&mut T` | 可变借用 | 读写 | **同时只能有一个**，且与 `&` 互斥 |

（约束的完整论证见 k1-3-4。）

引用的本质：**受编译器约束的指针**。它保证被指数据在被引用期间始终有效（不会悬垂），这是 C 指针与 Rust 引用的天壤之别。

## 三、解引用与自动解引用

```rust
let mut s = String::from("hi");
let r = &mut s;
r.push('!');        // 方法调用自动解引用（deref coercion）
(*r).push_str("!!"); // 显式解引用写法，效果相同
```

日常代码依赖自动解引用，几乎不需要手写 `*`；但理解它发生与否，是读懂签名（`&String` vs `&str`）的基础。

## 四、借用让 API 更通用

函数签名选择顺序（经验法则）：

1. 只读 → `&T`（或更抽象的 `&str`、`&[T]`）
2. 要修改 → `&mut T`
3. 要**持有/转移**（存进结构体、发到线程）→ `T`（拿所有权）

```rust
fn word_len(s: &str) -> usize { s.split_whitespace().next().map_or(0, str::len) }
// &str 是切片视图：&String 与 "literal" 都能传入（见 k1-3-5）
```

## 五、动手实验

```rust
fn grow(s: &mut String) { s.push_str("-grown"); }

fn main() {
    let mut s = String::from("a");
    grow(&mut s);
    println!("{s}");          // a-grown
    // let r1 = &s; let r2 = &mut s; println!("{r1} {r2}"); // ① E0502
}
```

打开注释行，读 E0502 报错：它会把冲突借用的**两处行号**都标出来——借用检查器的报错是全语言最友好的。

## 常见误区

- 「引用会拷贝数据」→ 不会，引用就是指针大小
- 「`&` 的变量本身也要 mut」→ 只有 `&mut` 需要；`&` 不可变借用即可
- 函数返回局部变量的引用 → E0515，所有权系统禁止悬垂，返回值请转移所有权或要求调用方传入缓冲

## 验证清单

- [ ] 把一个传 `String` 的函数改成传 `&String`/`&str` 并验证调用方变量仍可用
- [ ] 复现过 E0502
- [ ] 能说出三种参数选择的取舍

## 延伸阅读

- The Rust Book 4.2 — https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html
- 错误索引 E0502 — https://doc.rust-lang.org/error_codes/E0502.html
