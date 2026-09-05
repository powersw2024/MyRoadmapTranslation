# 结构体与方法实现

> 对应知识点：k1-4-1 · 预计 30 分钟 · 难度 ★☆☆

## 学习目标

- 用 struct 建模领域数据
- 在 impl 中区分关联函数与方法，选对 self 形式
- 掌握初始化简写与结构体更新语法

## 一、定义与实例化

```rust
struct Rectangle {
    width: f64,
    height: f64,
}

let mut r = Rectangle { width: 3.0, height: 4.0 };

// 字段名与变量同名时的初始化简写（field init shorthand）
let width = 5.0;
let r2 = Rectangle { width, height: 6.0 };

// 结构体更新语法（move 语义！）
let r3 = Rectangle { width: 1.0, ..r2 }; // height 从 r2 复制/移动
```

三种形态：

```rust
struct Unit;                    // 单元结构体：无数据（如标记类型）
struct Point(i32, i32);         // 元组结构体：命名字段类
struct Named { x: i32, y: i32 } // 常规结构体
```

## 二、impl：方法与关联函数

```rust
impl Rectangle {
    // 关联函数（无 self）＝其他语言的"静态方法"，常作构造器
    fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    fn area(&self) -> f64 {              // 只读方法
        self.width * self.height
    }

    fn scale(&mut self, factor: f64) {   // 可变方法
        self.width *= factor;
        self.height *= factor;
    }
}

let mut r = Rectangle::new(3.0, 4.0);
println!("{}", r.area());   // 12 ——方法调用自动引用/解引用
r.scale(2.0);
```

`self` 三形式的含义：

| 形式 | 语义 | 调用后原值 |
| --- | --- | --- |
| `&self` | 只读借用 | 仍可用（**默认选择**） |
| `&mut self` | 可变借用 | 仍可用，值被改 |
| `self` | 拿走所有权 | 失效（用于 `into_xxx` 类转换） |

## 三、derive(Debug) 与打印

```rust
#[derive(Debug)]
struct Rectangle { width: f64, height: f64 }

println!("{:?}", r);    // Rectangle { width: 3.0, height: 4.0 }
println!("{:#?}", r);   // 多行美化输出
```

不 derive 直接打印会报 E0277（`Rectangle doesn't implement Debug`）——这预告了 trait 约束的存在（k2-1-2）。

## 四、动手实验

实现 `Rectangle`：`new` 做参数校验（宽高 > 0，非法返回 `Option` 或 panic——两种方案都写并比较）、`area`、`can_hold(&self, other: &Rectangle) -> bool`。为每个方法写一个单测（提前预览 k4-1-2）。

## 常见误区

- 忘记 `&self` 导致方法吃掉所有权（E0382）
- 给所有方法都写 `self` → 90% 的情况应该是 `&self`
- 结构体更新语法 `..r2` 以为会保留 r2 → 不实现 Copy 时字段被 move

## 验证清单

- [ ] 实现了 new + 两个方法并写单测
- [ ] 能说出三种 self 形式的取舍
- [ ] 用过 `..` 更新语法并理解其所有权含义

## 延伸阅读

- The Rust Book 5.1/5.3 — https://doc.rust-lang.org/book/ch05-01-defining-structs.html
- Rust API Guidelines · naming — https://rust-lang.github.io/api-guidelines/naming.html
