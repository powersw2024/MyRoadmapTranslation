# 🦀 RustWay · 计算机学习之路

> 以 Rust 为主线的计算机科学系统教程 —— **编程 · 算法 · 数据库 · 网络 · Linux · 工程实践 · 测试**
> 知识图谱驱动 · 每个知识点可验证 · 内容与代码完全分离

![Rust](https://img.shields.io/badge/Rust-1.94+-DEA584?logo=rust)
![Vue](https://img.shields.io/badge/Vue-3-4FC08D?logo=vue.js)
![CI](https://github.com/powersw2024/rustway/actions/workflows/ci.yml/badge.svg)
![License](https://img.shields.io/badge/License-MIT-blue.svg)

## 这是什么

RustWay 是一个「教程即软件」的学习系统：

- **主页是一张交互式知识图谱** —— 10 个模块、43 个章节、178 个知识点、232 条前置依赖关系一目了然；你学过的知识点会亮起来，掌握的知识点带金色光环。学习成果直接映射在图上。
- **每个知识点都可验证** —— 详细讲解（持续填充中）、动手验证任务（在终端/测试中可客观完成）、自测题（先答后看解析）、官方引用出处。全部通过自测题 → 自动标记「已掌握」。
- **内容完整性由测试强制保障** —— `cargo test`（与 CI）会校验：知识点 id 唯一、章节引用存在、前置依赖无环、每个知识点必须有摘要/验证任务/自测题/http(s) 引用、被引用的文档真实存在。
- **任意插入知识点** —— 内容与代码分离：新建一个 `content/kps/*.json` 文件即自动入库（加载器合并目录下所有文件），在 `curriculum.json` 里把它挂到任意章节的任意位置；未编排的知识点会出现在「待编排」区，不会丢失。

## 模块总览

| # | 模块 | 内容 |
| - | ---- | ---- |
| 0 | 🧭 学习方法论 | 主动回忆/间隔重复、费曼技巧、刻意练习、环境搭建、复盘 |
| 1 | 🦀 Rust 编程基础 | 语法、控制流、**所有权系统**、结构体枚举、集合、错误处理 |
| 2 | ⚙️ Rust 进阶 | 泛型与 trait、生命周期、智能指针、并发、**async/await**、宏 |
| 3 | 🛠️ 工程实践 | Cargo/依赖管理、fmt/clippy、Git 协作、架构与设计模式 |
| 4 | ✅ 测试 | 单元/集成/文档测试、TDD、属性测试、基准、fuzz、CI 门禁 |
| 5 | 🧮 算法与数据结构 | 复杂度分析、线性结构、哈希、树与堆、图、DP、贪心回溯 |
| 6 | 🗄️ 数据库 | SQL、B+ 树索引、事务与隔离、rusqlite/sqlx、**参数绑定防注入**、NoSQL |
| 7 | 🌐 网络 | 分层模型、TCP/UDP、HTTP(S)/DNS、std 与 tokio 网络编程、Web 服务 |
| 8 | 🐧 Linux | 文件与权限、Shell、进程与信号、systemd、网络诊断、容器 |
| 9 | 🚀 综合实战 | 项目拆解、rustway 架构导读、参与开源、技术写作 |

课程设计遵循学习科学原则（见站内「科学学习方法」卡片）：讲解 → 示例 → 动手验证 → 自测回忆的主动学习闭环；章节按布卢姆分类学递进。

## 快速开始

依赖：Rust 1.85+ 与 Node.js 20+。

```bash
git clone https://github.com/powersw2024/rustway.git
cd rustway

# 构建前端（Vue 3 + Vite，产物在 frontend/dist）
cd frontend && npm install && npm run build && cd ..

# 运行（axum 同时提供 API 与静态站点）
cargo run --release
# ✓ 服务已启动: http://127.0.0.1:8080
```

打开 <http://127.0.0.1:8080> 即可开始学习。

前端开发模式（热更新）：

```bash
cd frontend && npm run dev   # http://localhost:5173，/api 自动代理到 8080
```

环境变量：`RUSTWAY_PORT`（默认 8080）、`RUSTWAY_CONTENT`（默认 `content`）、`RUSTWAY_STATIC`（默认 `frontend/dist`）。

## 架构

```text
content/                      ← 全部课程内容（纯数据，不懂 Rust 也能贡献）
├── curriculum.json           ← 模块/章节结构 + 关卡测验（kps 数组顺序 = 学习路径）
├── kps/*.json                ← 知识点定义（目录下所有文件自动合并，任意新增）
└── meta/ rust/ algo/ …/*.md  ← 知识点详细讲解（Markdown，pulldown-cmark 渲染）

frontend/                     ← Vue 3 + Vite 单页应用
└── src/
    ├── components/GraphCanvas.vue   ← sigma.js (WebGL) + ForceAtlas2 知识图谱
    ├── components/QuizWidget.vue    ← 自测题组件（即时反馈 + 解析）
    ├── views/                       ← 主页 / 模块页 / 知识点页
    └── composables/store.js         ← 路由 / 清单缓存 / 进度(localStorage) / 主题

src/                          ← Rust 后端（axum）
├── lib.rs                    ← 数据模型 + 加载 + 完整性校验 + 单元测试
├── api.rs                    ← API JSON 组装
└── main.rs                   ← /api/manifest · /api/kp/{id} · 静态托管

tests/content_integrity.rs    ← 内容完整性集成测试（CI 门禁）
```

数据流：`content/*.json` → 加载与校验（错误即测试失败）→ Markdown 预渲染 → API → 前端图谱与页面。
学习进度保存在浏览器 `localStorage`（键 `rustway.progress.v1`）。

## 添加一个知识点（无需改任何代码）

详见 [AUTHORING.md](AUTHORING.md)。最小流程：

1. 新建 `content/kps/my-kps.json`，写入知识点定义（含自测题与引用）；
2. 在 `content/curriculum.json` 中，把知识点 id 插入目标章节的 `kps` 数组任意位置（不插入则停留在「待编排」）；
3. `cargo test` 全绿即可提交 —— 测试会替你检查所有完整性约束。

## 质量门禁

```bash
cargo fmt --check          # 格式
cargo clippy -- -D warnings # lint
cargo test                 # 13+ 项测试：内容完整性 + 课程骨架规模 + API 一致性
cd frontend && npm run build
```

以上全部在 [.github/workflows/ci.yml](.github/workflows/ci.yml) 中强制执行。

## 路线图

- [x] 内容与代码分离的课程引擎 + 完整性测试
- [x] 知识图谱主页（sigma.js）+ 明暗主题
- [x] 测验 / 掌握状态 / 进度持久化
- [ ] 服务端进度存储（SQLite + 参数绑定，呼应第 6 模块）
- [ ] 间隔重复复习队列（到期知识点推荐）
- [ ] 全部 178 个知识点的详细讲解文档填充（当前首批 24 篇已就位，大纲/任务/测验已全量覆盖）

## License

MIT
