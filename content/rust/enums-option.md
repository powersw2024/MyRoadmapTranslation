# 枚举与 Option

> 对应知识点：k1-4-2 · 预计 30 分钟 · 难度 ★★☆

## 学习目标

- 用携带数据的枚举精确建模状态
- 解释 Option<T> 如何消灭「忘记判空」这类 bug
- 熟练使用 unwrap_or / map 等组合子

## 一、枚举：携带数据的变体

Rust 的枚举是**代数数据类型（和类型）**：每个变体可以携带不同结构的数据。

```rust
enum Shape {
    Circle { radius: f64 },          // 结构体式变体
    Rect(f64, f64),                  // 元组式变体
    Point,                           // 无数据变体
}

fn area(s: &Shape) -> f64 {
    match s {
        Shape::Circle { radius } => std::f64::consts::PI * radius * radius,
        Shape::Rect(w, h) => w * h,
        Shape::Point => 0.0,
    }
}
```

`match`（k1-2-3/k1-4-3）与枚举是一对：**穷尽性检查保证新增变体时所有处理点都被编译器点名**。

## 二、Option<T>：类型化的「可能没有」

标准库定义：

```rust
enum Option<T> {
    Some(T),
    None,
}
```

对比其他语言：

```rust
// Java/JS：任何引用都可能是 null，运行时 NullPointerException 随时爆炸
// Rust：T 与 Option<T> 是不同类型，想当 T 用？编译器要求先证明不是 None
let maybe: Option<i32> = find_user(42);

let name = maybe.map(|id| id * 2).unwrap_or(0);  // 组合子风格
match maybe {                                     // 完整处理风格
    Some(v) => println!("found {v}"),
    None => println!("missing"),
}
```

核心收益：**null 的存在性从运行时事故变成编译期信息**。函数签名 `fn find(&self, id: u32) -> Option<&User>` 一眼说明「可能查无此人」，调用者无法绕过。

## 三、常用组合子速查

```rust
let o: Option<i32> = Some(3);
o.unwrap();          // 3；None 则 panic（原型/测试用）
o.expect("说明");    // 同上但 panic 带原因（更好）
o.unwrap_or(0);      // None 时给默认值
o.unwrap_or_default(); // 默认类型的零值
o.map(|v| v * 2);    // Some(f(x)) / None 保持 None
o.and_then(|v| f(v)); // 链式返回 Option 的函数（flatMap）
o.ok_or("reason")?;  // 转 Result，进入错误传播（k1-6-3）
```

## 四、动手实验

1. 定义 `enum Token { Number(f64), Op(char), LParen, RParen }`，写 `fn tokenize(input: &str) -> Vec<Token>` 的简化版（仅数字与 + - * / ( )）
2. `fn div(a: f64, b: f64) -> Option<f64>`：除零返回 None；调用处分别用 unwrap_or 与 match 处理
3. 把 2 中的 Option 改成 Result<f64, String>（预览 k1-6-2），对比错误信息能力

## 常见误区

- 到处 `.unwrap()` → 原型可以，生产代码应把 None 变成有意义的错误或默认值
- 用 `==` 比较 Option 与 None 需要注意类型一致性 → 更惯用 `is_none()`/`is_some()`
- 把「枚举当常量集」用惯了，忘记变体可以带数据 → 这是 Rust 枚举的真正威力

## 验证清单

- [ ] 写过一个多变体带数据的枚举 + match
- [ ] 能向他人解释 Option 消灭空指针的机制
- [ ] 用 unwrap_or/map 改写过一段 match 嵌套

## 延伸阅读

- The Rust Book 6.1 — https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html
- std::option 官方模块文档（组合子大全）— https://doc.rust-lang.org/std/option/
