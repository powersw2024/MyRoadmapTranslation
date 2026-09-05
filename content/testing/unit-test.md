# #[test] 与断言

> 对应知识点：k4-1-2 · 预计 25 分钟 · 难度 ★☆☆

## 学习目标

- 写出第一组单元测试并解读测试输出
- 掌握 assert 系列与 Result 风格测试

## 一、最小闭环

```rust
fn add(a: i32, b: i32) -> i32 { a + b }

#[cfg(test)]
mod tests {
    use super::*;               // 引入被测项

    #[test]
    fn adds_positive_numbers() {
        assert_eq!(add(2, 3), 5);
    }

    #[test]
    fn adds_negatives_and_zero() {
        assert_eq!(add(-1, 1), 0);
        assert_eq!(add(0, 0), 0);
    }
}
```

`cargo test` 输出：

```text
running 2 tests
test tests::adds_negative ... ok
test tests::adds_positive_numbers ... ok

test result: ok. 2 passed; 0 failed; 0 ignored
```

要点：

- `#[cfg(test)]`：测试代码**只在 cargo test 时编译**，不进发布产物
- 测试函数名 = 行为描述（英文惯用：被测行为_条件_预期）
- 默认**并行**运行，测试之间必须相互独立

## 二、断言家族

| 宏 | 用途 | 失败输出 |
| --- | --- | --- |
| `assert!(cond)` | 布尔条件 | 条件表达式 |
| `assert_eq!(a, b)` | 相等（需 PartialEq + Debug） | **两个值 + 行号** |
| `assert_ne!(a, b)` | 不等 | 两个值 + 行号 |

`assert_eq!` 的失败信息直接展示左右值，调试成本最低——**能用 eq/ne 就不用裸 assert!**。

## 三、验证 panic 与 Result 风格

```rust
#[test]
#[should_panic(expected = "index out of bounds")]
fn panics_on_bad_index() {
    let v = vec![1];
    let _ = v[99];
}

#[test]
fn with_result() -> Result<(), String> {
    let n: i32 = "42".parse().map_err(|e| e.to_string())?;
    assert_eq!(n, 42);
    Ok(())      // Err 提前返回 = 测试失败
}
```

Result 风格适合 setup 阶段多用 `?` 的场景；断言仍用 assert 系列。

## 四、动手实验

给 `fn is_palindrome(s: &str) -> bool` 写 5 个测试：空串、单字符、奇数长度回文、非回文、含 Unicode（"上海自来水来自海上"）。故意写错一个预期值，观察失败输出如何定位问题。

## 验证清单

- [ ] 完成回文测试组且全绿
- [ ] 制造过一次 assert_eq! 失败并读懂输出
- [ ] 用 should_panic 验证过 panic 路径

## 延伸阅读

- The Rust Book 11.1 — https://doc.rust-lang.org/book/ch11-01-writing-tests.html
- std 测试参考 — https://doc.rust-lang.org/reference/attributes/testing.html
