# docs → 知识点 JSON 转换规范（docs2kps）

目标：把仓库外 `docs/` 资料库（5 个仓库，1122 个 Markdown 文件）按知识点结构
全部转换为 `content/kps/*.json`，全部中文；编程类知识点统一转换为 **Rust 与 Python**
双语言示例；每个知识点可验证、可溯源。

## 流程（每个源文件）

1. 读取 `docs/<repo>/<path>.md`，提炼知识点（中文撰写）。
2. 生成输出文件 `content/kps/<prefix>-<hash6>.json`：
   ```bash
   python3 tools/kpkit.py id-for <prefix> <docs相对路径>   # → id \t 输出文件名
   ```
3. 编程示例必须先验证再写入：
   - Python：`python3` 实际运行；
   - Rust：`rustc --edition 2021` 编译运行（或放临时目录 `rustc x.rs && ./x`）；
   - 非显而易见的结论（输出值、边界行为）以运行结果为准。
4. 写出 JSON（结构 `{"kps": [...]}`，字段遵循 `schema/kp.schema.json`）。
5. 登记 + 校验 + 提交（**每完成一个文件提交一次**）：
   ```bash
   python3 tools/kpkit.py register content/kps/<file>.json <docs相对路径>
   cargo run --offline -- validate
   git add <output> tools/docs2kps-manifest.json && git commit -m "content(<prefix>): <主题>"
   ```

## 命名与 ID

- 前缀：`cn`=CS-Notes，`jg`=JavaGuide，`sg`=CSStudyGuide，`sl`=cs-self-learning，`fb`=free-programming-books。
- `id = <prefix>-<sha256(docs相对路径) base36 前 8 位>`（同一源文件拆多个知识点时追加 `-2`、`-3`…）。
- `id` 全局唯一且确定性派生，无需中央计数器；输出文件名同源（前 6 位）。

## 知识点质量要求

- `title` / `summary` / `outline` / `task` / `quiz` 全部中文（专有名词保留原文）。
- 编程类：`outline` 中同一算法/概念给出 Rust 与 Python 两种实现，代码经过运行验证。
- `task`：写清「做什么 + 如何客观确认做对了」（命令输出 / 通过的测试 / 产物）。
- `refs`：≥1 条 http(s)。来源文件必须注明——引用 `t` 形如
  `来源：docs/CS-Notes-master/notes/24. 反转链表.md`，`u` 用上游仓库对应 blob 链接
  （CS-Notes=master，JavaGuide=main，cs-self-learning=master，free-programming-books=main）；
  官方文档链接优先补充（Rust Book、Python 文档等）。
- `quiz` ≥1 题；编程知识点优先 code（Rust，走 rustway 判题）/blank/subjective。
- `prereqs` 只引用已存在（已提交或同文件）的 id；不得成环。
- `tags` 2~5 个中文标签，参与相似算法自动关联。

## 覆盖率账本

`tools/docs2kps-manifest.json` 登记每个源文件的输出、KP ids 与 sha256。
`python3 tools/kpkit.py coverage` 打印未覆盖文件清单——全部转换完成的验收标准：

```bash
python3 tools/kpkit.py coverage    # 覆盖 1122/1122，无未覆盖输出
```

## 仓库对应

| 前缀 | docs 仓库 | 上游 | 分支 |
| --- | --- | --- | --- |
| cn | CS-Notes-master | CyC2018/CS-Notes | master |
| jg | JavaGuide-main | Snailclimb/JavaGuide | main |
| sg | CSStudyGuide-main | 来源不明（本地资料） | — |
| sl | cs-self-learning-master | PKUFlyingPig/cs-self-learning | master |
| fb | free-programming-books-main | EbookFoundation/free-programming-books | main |
