# println! 与格式化体系

> 对应知识点：k1-1-5 · 预计 20 分钟 · 难度 ★☆☆

## 学习目标

- 熟练使用捕获标识符、位置参数与格式规格
- 理解 Display 与 Debug 两套输出体系
- 解释为什么 println! 是宏

## 一、为什么 println! 是宏

`println!("hi {}", x)` 在编译期**解析格式串本身**：参数个数、类型是否实现对应 trait 都在编译期检查。函数做不到这一点（函数的格式串只是普通字符串参数）。这也是为什么格式串写错（如 `println!("{} {}", 1)`）是**编译错误**而不是运行时事故——C 的 printf 族做不到。

## 二、四种插值写法

```rust
let name = "rustway";
let score = 97;

// 1. 捕获标识符（最常用，Rust 2021+）
println!("{name} 得分 {score}");

// 2. 位置参数
println!("{0} 再次出场 {0}，另一个 {1}", name, score);

// 3. 命名参数
println!("{subject}: {score}", subject = "算法");

// 4. 表达式（花括号内可直接放表达式）
println!("下一课: {}", score + 1);
```

## 三、格式规格：宽度、对齐、填充、精度

```rust
let n = 42;
println!("[{n:5}]");     // [   42] 右对齐，宽 5
println!("[{n:<5}]");    // [42   ] 左对齐
println!("[{n:^5}]");    // [ 42  ] 居中
println!("[{n:05}]");    // [00042] 零填充
println!("[{n:+}]");     // [+42] 正号
let pi = 3.14159;
println!("[{pi:.2}]");   // [3.14] 小数精度
println!("[{n:b}]");     // 101010 进制：x/X/o/b/e
```

## 四、Display 与 Debug 两套体系

| trait | 触发写法 | 面向 | 缺失时 |
| --- | --- | --- | --- |
| `Display` | `{}` | 终端用户，优雅展示 | 编译错误 E0277 |
| `Debug` | `{:?}` / `{:#?}` | 程序员，调试信息 | 编译错误 E0277 |

```rust
#[derive(Debug)]
struct Point { x: i32, y: i32 }

let p = Point { x: 1, y: 2 };
// println!("{}", p);   // ❌ Point 没实现 Display
println!("{:?}", p);    // Point { x: 1, y: 2 }
println!("{:#?}", p);   // 多行美化版
```

标准库类型大多两者都有；自定义类型 `derive(Debug)` 即可，Display 需要手写 impl（k2-1-2 的 trait 练习）。

## 五、写文件与字符串：write! 家族

`println!` = `print!` + 换行。同族宏：

```rust
use std::fmt::Write;
let mut s = String::new();
write!(s, "{}-{}", 1, 2).unwrap();   // 写入 String
// io::Write 的 write! 写文件/socket，同名不同 trait
```

## 动手实验

1. 打印一张 3 行对齐的"成绩表"（名字左对齐 10 宽、分数右对齐 3 宽零填充）
2. derive Debug 打印 `Vec<HashMap<&str, i32>>` 的 `{:#?}`
3. 制造一次 Display 缺失的 E0277 并读懂报错建议（提示里通常直接给出"consider deriving Debug"）

## 常见误区

| 误区 | 事实 |
| --- | --- |
| 格式串与参数个数不符只报警告 | 是编译错误，宏在编译期检查 |
| 以为 `{}` 能打印一切 | 必须实现 Display；调试用 `{:?}` |
| 在 release 用户界面用 `{:?}` | Debug 输出是给开发者的，含转义与引号 |

## 验证清单

- [ ] 用过 4 种插值写法各一次
- [ ] 打印过零填充/对齐的表格
- [ ] 解释 Display 与 Debug 的面向差异

## 延伸阅读

- std::fmt — https://doc.rust-lang.org/std/fmt/
- The Rust Book 3.1（println! 示例）— https://doc.rust-lang.org/book/ch03-01-variables-and-mutability.html
