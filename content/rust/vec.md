# Vec 动态数组

> 对应知识点：k1-5-1 · 预计 30 分钟 · 难度 ★☆☆

## 学习目标

- 熟练完成 Vec 的增删改查与三种遍历
- 在 `[]` 与 `.get()` 之间做出正确选择
- 理解 push 的所有权转移

## 一、创建与增删

```rust
let mut v: Vec<i32> = Vec::new();
let mut w = vec![1, 2, 3];        // vec! 宏（最常用）
let z = vec![0; 10];              // 10 个 0
w.push(4);                        // 尾部追加，均摊 O(1)（k5-1-3）
let last = w.pop();               // Option<i32>：可能为空！
w.insert(0, 0);                   // 头部插入 O(n)：搬移全部元素
w.remove(0);                      // 删除并返回该元素
```

性能直觉：**Vec 是尾部优化结构**——push/pop O(1) 均摊，随机访问 O(1)，中间插删 O(n)。需要头部操作用 `VecDeque`。

## 二、读取：[] vs get()

```rust
let v = vec![10, 20, 30];
let a = v[5];        // ❌ panic: index out of bounds
let b = v.get(5);    // ✅ Option<&i32> = None
```

- `[]`：确定不会越界时（例如循环内合法下标），越界即 panic 属于程序 bug
- `.get()`：下标可能越界是**正常业务路径**时（用户输入、分页参数）

这个选择本身就是「可恢复 vs 不可恢复错误」的预演（k1-6-1/k1-6-2）。

## 三、遍历的三种形式（所有权视角）

```rust
for x in &v {           // 不可变借用：v 之后仍可用，x: &i32
    println!("{x}");
}
for x in &mut v {       // 可变借用：可修改元素
    *x *= 2;
}
for x in v {            // 消耗：v 的所有权转移到循环，之后 v 不可用
    consume(x);
}
```

判断标准：**之后还要用 v 吗？** 要 → `&`/`&mut`；不要（转换/收集成新集合）→ 直接消耗。

## 四、所有权细节：push 会 move

```rust
let s = String::from("hi");
let mut vs = Vec::new();
vs.push(s);
// println!("{s}");   // ❌ E0382：字符串所有权已进入 vec
```

Vec 只**拥有**元素本身；想让多个 Vec 共享同一数据，需要 `Rc`（k2-3-3）或存引用（受生命周期约束）。

## 五、动手实验：待办列表 CLI

```rust
use std::io::{self, BufRead};

enum Cmd { Add(String), List, Quit }

fn main() {
    let mut todos: Vec<String> = Vec::new();
    let stdin = io::stdin();
    for line in stdin.lock().lines().map_while(Result::ok) {
        match parse(&line) {
            Cmd::Add(t) => todos.push(t),
            Cmd::List => for (i, t) in todos.iter().enumerate() {
                println!("{i}: {t}");
            },
            Cmd::Quit => break,
        }
    }
}
```

自行实现 `parse`（提示：`split_once(' ')` 返回 Option），并用 `cargo run` 实测增查删。

## 常见误区

- 迭代中 `v.push(..)` → E0502，借用冲突（先收集到新 Vec 或先迭代完）
- 以为 `pop()` 返回元素 → 是 `Option`，必须处理空情况
- 用 `Vec<T>` 存大对象频繁中间插删 → 考虑 `VecDeque`/`LinkedList` 或重设数据结构

## 验证清单

- [ ] 完成 Todo CLI 的 add/list/remove
- [ ] 触发过 index out of bounds panic 并改用 get
- [ ] 解释三种 for 循环形式的区别

## 延伸阅读

- The Rust Book 8.1 — https://doc.rust-lang.org/book/ch08-01-vectors.html
- std::vec::Vec（方法大全）— https://doc.rust-lang.org/std/vec/struct.Vec.html
