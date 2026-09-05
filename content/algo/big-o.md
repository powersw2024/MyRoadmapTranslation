# 大 O 记号

> 对应知识点：k5-1-1 · 预计 30 分钟 · 难度 ★☆☆

## 学习目标

- 给一段代码估算时间复杂度
- 用实测数据验证复杂度推算

## 一、定义

大 O 描述**增长率上界**：当 n 足够大时，`f(n) ≤ c·g(n)`（存在常数 c 与 n₀），记作 `f(n) = O(g(n))`。

实践含义：**只保留增长最快的项，忽略常数**。

- `3n² + 100n + 500` → `O(n²)`
- `2n log n + 5` → `O(n log n)`

## 二、常见等级速查

| 复杂度 | n=10⁶ 时操作量级 | 典型例子 |
| --- | --- | --- |
| O(1) | 1 | 数组寻址、HashMap 查 |
| O(log n) | ~20 | 二分查找 |
| O(n) | 10⁶ | 线性扫描 |
| O(n log n) | ~2×10⁷ | 排序 |
| O(n²) | 10¹² | 双重循环（不可接受） |
| O(2ⁿ) | 天文数字 | 朴素子集枚举 |

经验基准：1 秒约能完成 10⁸ 次基本操作——据此在动手前估算算法是否可行。

## 三、快速估算法

```rust
for i in 0..n {            // O(n)
    for j in 0..n {        //   O(n) → 嵌套相乘 = O(n²)
        // O(1) 工作
    }
}
for i in 0..n {            // O(n)
    let mut j = i;
    while j > 0 { j /= 2; } //   O(log n) → O(n log n)
}
```

规则：顺序相加取最大；嵌套相乘。

## 四、实测验证（本知识点核心：可验证）

```rust
use std::time::Instant;

fn linear_find(v: &[i32], x: i32) -> bool { v.iter().any(|&e| e == x) }
fn binary_find(v: &[i32], x: i32) -> bool { // v 已排序
    let (mut lo, mut hi) = (0usize, v.len());
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        match v[mid].cmp(&x) { std::cmp::Ordering::Less => lo = mid + 1, Equal => return true, Greater => hi = mid }
    }
    false
}

fn main() {
    for n in [100_000usize, 1_000_000] {
        let v: Vec<i32> = (0..n as i32).collect();
        let t = Instant::now(); let _ = linear_find(&v, -1);
        println!("linear  n={n}: {:?}", t.elapsed());
        let t = Instant::now(); let _ = binary_find(&v, -1);
        println!("binary  n={n}: {:?}", t.elapsed());
    }
}
```

观察：n 扩大 10 倍，linear 耗时约 ×10（O(n)），binary 几乎不变（O(log n)，约 +3 次比较）。

## 常见误区

- 把大 O 当精确耗时 → 它描述趋势，常数因子在工程中也很重要
- 忽略最坏情况 → HashMap 平均 O(1) 但最坏 O(n)（攻击者可制造聚集）
- 只算时间不算空间 → 见 k5-1-4

## 验证清单

- [ ] 完成实测实验并解释两条曲线
- [ ] 估算一段三层嵌套循环的复杂度
- [ ] 说出 n=10⁵ 时 O(n²) 为什么不可行

## 延伸阅读

- Khan Academy · Asymptotic Notation — https://www.khanacademy.org/computing/computer-science/algorithms/asymptotic-notation/a/asymptotic-notation
- CSES 竞赛程序设计手册（中文版）第 2 章 — https://cses.fi/book/book.pdf
