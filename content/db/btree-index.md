# 索引原理：B+ 树

> 对应知识点：k6-2-1 · 预计 35 分钟 · 难度 ★★☆

## 学习目标

- 解释为什么 B+ 树适合磁盘存储
- 用 EXPLAIN 验证索引命中与否

## 一、没有索引的世界

`WHERE name = 'abc'` 无索引时只能**全表扫描**：逐行读取比较，O(n)。100 万行的表 = 100 万次行访问。

## 二、B+ 树结构

```text
                 [10 | 40]                ← 内部节点只存键（路标）
               /    |    \
        [3|7]    [12|25]   [42|88]        ← 内部节点
        / | \     ...        ...   \
   叶子: [1,3,5,7] ↔ [10,12,15,25] ↔ [40,42,...]  ← 数据在叶子，叶子成链
```

三个关键性质决定了它的地位：

1. **矮胖多叉**：每节点存上百个键 → 100 万行树高仅 2~3 层 → 每次查找只需 2~3 次**磁盘块 IO**（IO 是数据库最贵操作）
2. **叶子链表**：范围查询 `WHERE id BETWEEN 10 AND 40` 沿链顺序扫描，无需回到树上
3. **有序**：支持 `>` `<` `BETWEEN` `ORDER BY` 与最左前缀

对比哈希索引：O(1) 等值查询快，但**不支持范围与排序**，所以数据库索引默认 B+ 树。

## 三、最左前缀原则

联合索引 `(a, b, c)` 的键按 a 排序、a 相同按 b、再按 c：

- `WHERE a=1 AND b=2` ✅ 走索引
- `WHERE b=2` ❌ 跳过 a，无法利用有序性
- `WHERE a=1 AND c=3` ⚠️ 只用到 a

**索引失效常见场景**：`LIKE '%abc'`（前置通配符破坏有序性）、对列套函数 `WHERE upper(name)=...`、隐式类型转换。

## 四、动手实验（SQLite）

```sql
CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT);
-- 插入 100 万行后：
EXPLAIN QUERY PLAN SELECT * FROM t WHERE name = 'user500000';
-- 输出 SCAN t（全表扫描）
CREATE INDEX idx_name ON t(name);
EXPLAIN QUERY PLAN SELECT * FROM t WHERE name = 'user500000';
-- 输出 SEARCH t USING INDEX idx_name（索引查找）
```

用 `.timer on` 对比两种情况耗时（通常差 2~3 个数量级），再验证 `LIKE '%abc'` 与 `LIKE 'abc%'` 的计划差异。

## 五、索引的代价

- **写放大**：每次 INSERT/UPDATE 同步维护树结构
- **存储**：索引本身占磁盘
- 原则：为高频查询的列建索引，不为低频/写密集表滥建

## 验证清单

- [ ] 画出 B+ 树结构并解释叶子链表的作用
- [ ] 用 EXPLAIN QUERY PLAN 复现 SCAN→SEARCH 的转变
- [ ] 复现一次 LIKE 前置通配符导致的索引失效

## 延伸阅读

- SQLite EXPLAIN QUERY PLAN — https://www.sqlite.org/eqp.html
- Use The Index, Luke（索引原理圣经）— https://use-the-index-luke.com/
