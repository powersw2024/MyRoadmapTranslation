# ? 运算符

> 对应知识点：k1-6-3 · 预计 25 分钟 · 难度 ★★☆

## 学习目标

- 展开写 ? 的等价 match
- 用 From 让多个错误类型统一

## 一、? 做了什么

```rust
fn read_username() -> Result<String, io::Error> {
    let mut s = String::new();
    File::open("username.txt")?.read_to_string(&mut s)?;
    Ok(s.trim().to_string())
}
```

`expr?` 等价于：

```rust
match expr {
    Ok(v) => v,
    Err(e) => return Err(From::from(e)),  // 注意 From::from！
}
```

两个关键点：Ok 解包继续；Err **先经 From 转换**再提前返回——这就是不同错误类型能自动统一的原因。

## 二、From 驱动的错误归并

```rust
enum AppError { Io(io::Error), Parse(std::num::ParseIntError) }

impl From<io::Error> for AppError {
    fn from(e: io::Error) -> Self { AppError::Io(e) }
}
impl From<std::num::ParseIntError> for AppError {
    fn from(e: std::num::ParseIntError) -> Self { AppError::Parse(e) }
}

fn load() -> Result<i32, AppError> {
    let text = std::fs::read_to_string("n.txt")?;  // io::Error 自动转 AppError
    text.trim().parse::<i32>()?;                    // ParseIntError 自动转
    // ...
    Ok(0)
}
```

## 三、使用条件

- 函数返回 `Result`（或 Option——? 对 None 直接返回 None）时才能用
- 在 `main` 返回 `Result<(), Box<dyn Error>>` 时也可用

## 四、动手实验

1. 把 k1-6-2 的 match 版读文件函数改写为 ? 版，对比行数
2. 定义两变体错误枚举 + 两个 From 实现，让 ? 跨两种来源错误
3. 在不返回 Result 的函数里用 ?，抄录编译错误 E0277

## 验证清单

- [ ] 能默写 ? 的等价展开
- [ ] 实现过一次 From 转换
- [ ] 体验过「错误类型不一致导致 ? 失败」的报错

## 延伸阅读

- The Rust Book · The ? Operator — https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html#a-shortcut-for-propagating-errors-the--operator
- std::convert::From — https://doc.rust-lang.org/std/convert/trait.From.html
