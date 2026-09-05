# loop/while/for 三种循环

> 对应知识点：k1-2-2 · 预计 20 分钟 · 难度 ★☆☆

## 学习目标

- 按场景在三种循环中正确选择
- 掌握 loop 返回值与循环标签两个 Rust 特性

## 一、loop：专用无限循环

```rust
let mut count = 0;
let result = loop {
    count += 1;
    if count == 10 {
        break count * 2;    // break 可携带值 → 整个 loop 表达式的值
    }
};
assert_eq!(result, 20);
```

为什么有了 `while true` 还要 loop：

1. 编译器**知道** loop 至少执行一次且只能靠 break 退出 → 可对 `break value` 做类型检查（`while true` 做不到，其类型永远是 `()`）
2. 意图明确：loop 读作"循环直到内部决定退出"

重试类逻辑的典型形态：

```rust
let conn = loop {
    match try_connect() {
        Ok(c) => break c,
        Err(e) => {
            if attempts_exhausted() { panic!("放弃: {e}") }
            sleep_backoff();
        }
    }
};
```

## 二、while：条件驱动的循环

```rust
let mut n = 100;
while n > 1 {
    n = if n % 2 == 0 { n / 2 } else { 3 * n + 1 };
}
```

注意：`while cond` 循环体**不能**用 break 携带值（类型系统限制）。

## 三、for：遍历一切迭代器

```rust
for i in 0..5 { }            // Range: 0 1 2 3 4
for i in (0..=5).rev() { }   // 倒序闭区间
for x in &vec { }            // 借用遍历（k1-5-4 详述所有权差异）
for (i, x) in vec.iter().enumerate() { }  // 带下标
for (k, v) in map.iter() { } // 键值对
```

Rust 的 for **不是** C 的计数循环，而是对 `IntoIterator` 的语法糖——因此永远不会有"差一错误"的边界判断，也没有 `arr.length - 1` 这类手写下标。

## 四、循环标签：一次跳出多层

```rust
'outer: for x in 0..5 {
    for y in 0..5 {
        if x * y > 6 {
            break 'outer;     // 直接跳出外层
        }
        if y == 2 { continue 'outer; }
    }
}
```

标签 `'` 开头，放在循环语句前。比"旗标变量 + 每层 if 检查"清晰得多。

## 五、选择指南

| 场景 | 用 |
| --- | --- |
| 重试直到成功/退出条件在循环内部 | `loop` |
| 先验条件驱动（如收敛阈值） | `while` |
| 遍历集合/区间（99% 的情况） | `for` |

## 动手实验

1. 猜数字：loop + break 返回尝试次数
2. 用 for 与 `.rev()` 打印 10..=1 倒计时
3. 在 5×5 网格中搜索第一个 `x*y > 6` 的坐标，用标签一次跳出并打印
4. 故意在 while 里 `break 5`，读编译错误（E0571）

## 常见误区

| 误区 | 事实 |
| --- | --- |
| 用 `while true` | 编译器建议改 loop；且无法 break 出值 |
| 手写下标 `for i in 0..v.len()` 再 `v[i]` | 直接 `for x in &v`；需要下标用 enumerate |
| 嵌套循环用布尔旗标层层退出 | 用 `'label` |

## 验证清单

- [ ] 用 loop 的 break 值写过重试逻辑
- [ ] 用过 enumerate 与 rev
- [ ] 用标签跳出过双层循环

## 延伸阅读

- The Rust Book 3.5 · Loops — https://doc.rust-lang.org/book/ch03-05-control-flow.html
- 错误索引 E0571（while 中 break 值）— https://doc.rust-lang.org/error_codes/E0571.html
