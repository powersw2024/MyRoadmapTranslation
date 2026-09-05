# 参数绑定防 SQL 注入

> 对应知识点：k6-3-2 · 预计 30 分钟 · 难度 ★★☆

## 学习目标

- 复现一次注入攻击以理解原理
- 把所有动态查询改写为参数绑定

## 一、注入的原理

拼接 SQL 时，**用户输入成为 SQL 代码的一部分**：

```rust
// ❌ 危险：拼接
let q = format!("SELECT * FROM users WHERE name = '{}' AND pw = '{}'", name, pw);
// 输入 name = "admin' --"
// 生成: SELECT * FROM users WHERE name = 'admin' --' AND pw = '...'
// -- 注释掉密码校验 → 无需密码登录任意用户！
// 更狠的: "'; DROP TABLE users; --"（破坏性注入）
```

## 二、参数绑定：结构与数据分离

```rust
// ✅ rusqlite 参数绑定
let mut stmt = conn.prepare("SELECT id FROM users WHERE name = ?1 AND pw = ?2")?;
let rows = stmt.query_map(params![name, pw], |r| r.get::<_, i64>(0))?;
```

预编译语句先把 **SQL 骨架（AST）固定**，参数只填充数据槽位——无论输入什么字符，都不可能改变语句结构。`'` 不再是字符串边界，只是普通字符。

**这条规则没有例外**：任何来自外部的输入（HTTP 参数、文件内容、配置）进入 SQL 都必须绑定。

## 三、占位符覆盖不到的地方

表名/列名/排序方向是**标识符**，不能当数据绑定：

```rust
// ❌ 拼接（注入面）
format!("SELECT * FROM t ORDER BY {}", user_sort);
// ✅ 白名单映射
let col = match user_sort.as_str() {
    "name" => "name",
    "created" => "created_at",
    _ => "id", // 默认值兜底
};
format!("SELECT * FROM t ORDER BY {col}");
```

## 四、动手实验

1. 建一个 users 表 + 一条用户数据
2. 写拼接版登录函数，用 `admin' --` 攻破它（打印生成的 SQL 观察语义被篡改）
3. 改写为 `?1` 绑定版，同样的输入返回「用户不存在」
4. 把「本项目所有 SQL 一律参数绑定，标识符走白名单」写入团队规范文档

## 验证清单

- [ ] 复现过注入并解释 -- 注释的作用
- [ ] 改写过绑定版并验证攻击失效
- [ ] 为动态排序实现白名单

## 延伸阅读

- OWASP SQL Injection Prevention Cheat Sheet — https://cheatsheetseries.owasp.org/cheatsheets/SQL_Injection_Prevention_Cheat_Sheet.html
- rusqlite params 文档 — https://docs.rs/rusqlite/latest/rusqlite/
