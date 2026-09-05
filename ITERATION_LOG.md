# ITERATION_LOG — rustway 迭代演进日志

> 每轮迭代一个条目：分析摘要 → 本轮任务 → 实施勾选 → 测试结果 → 提交状态。

---

## Iteration #1 — 2026-09-06T00:20+08:00

状态：✅ DONE（提交 a527778）

### 分析摘要（7 维度）

| 维度 | 评估 | 说明 |
| ---- | ---- | ---- |
| 1. 架构与代码质量 | ✅ 良好 | 内容（content/ JSON）/ 引擎（src/）/ 前端（frontend/ Vue）彻底分离；db.rs 单一持久化层、ai.rs 统一出站通道；命名与注释规范 |
| 2. 功能完整性 | ⚠️ 可改进 | **工作区存在上一轮中断的半成品**：AI 配置持久化——后端 60%（`ai_config` 表 + GET/POST `/api/ai/config` 已就位，基线可编译、34 测试全绿）；断点：`effective_ai()` 仍只读环境变量（保存的配置不生效）、前端 `AiSettingsView.vue` 仍是纯只读页（无表单、不识别 `source:"database"`） |
| 3. 健壮性与错误处理 | ✅ 良好 | DB 失败不阻塞服务（健康检查如实上报）、AI 不可用优雅降级自评、judge 沙箱编译、保存接口有 backend/base_url/model 全量校验 |
| 4. 安全性 | ✅ 良好 | SSRF 双重校验 + DNS 钉扎 + 禁重定向；SQL 全参数绑定；密钥只入库不回传不入日志；无硬编码凭据 |
| 5. 性能与可观测性 | ⚠️ 可改进 | gzip/br + TraceLayer 已有；无指标埋点（当前规模不构成瓶颈，后续轮次） |
| 6. 测试覆盖 | ⚠️ 可改进 | 27 单测 + 7 集成测试全绿，内容完整性门禁强；**新增的 `save_ai_config`/`get_ai_config_full` 无测试**；前端无测试基建（项目规模下可接受） |
| 7. 工程化与文档 | ✅ 良好 | CI 双 job（fmt/clippy/test + vite build）；README/AUTHORING.md 详实；缺 ITERATION_LOG.md（本轮创建） |

**幂等性检查**：工作区有 `src/db.rs`、`src/main.rs` 未提交改动（上述半成品）。经评估：改动连贯、基线门禁（fmt/clippy/34 测试）全绿、与 README 路线图及 `review` 接口错误提示（"请在「AI 设置」页配置"）方向一致。**决策：不回滚，将补齐该半成品作为本轮任务主体**，避免丢弃已完成后端工作。

### 本轮任务

- [x] 任务 1：补齐 AI 配置持久化半成品（影响面大 × 成本中）
  - 目标：设置页保存的配置真正生效（DB 优先、环境变量回退）；前端设置页支持编辑与保存。
  - 涉及文件：`src/main.rs`（`effective_ai` 单函数改造）、`frontend/src/views/AiSettingsView.vue`（表单化改造）
  - 验收：`cargo fmt --check` / `cargo clippy --all-targets -- -D warnings` / `cargo test` 全绿；`npm run build` 成功；冒烟：POST 保存后 GET 返回 `source:"database"`，`/api/ai/latency` 使用 DB 配置；安全契约不回退（出站 URL 仍只来自服务端存储/环境变量，`guarded_client` SSRF 双重校验不变）
- [x] 任务 2：补齐 `ai_config` 持久化单元测试（测试盲区）
  - 涉及文件：`src/db.rs`（仅 `#[cfg(test)]` 模块追加）
  - 验收：新增 ≥2 个测试覆盖 save→get 往返、api_key 保留/清除语义，`cargo test` 全绿
- [x] 任务 3：文档同步
  - 涉及文件：`README.md`
  - 验收：README 中 AI 配置描述与实际行为一致（DB 优先 + 环境变量回退）

变更半径控制：核心逻辑仅改 `src/main.rs::effective_ai` 一个函数（≤3 文件约束内）。

### 实施记录

1. `src/main.rs`：新增 `db_ai_config()`（读库 + backend 白名单 + `validate_base_url` 深度防御，防库文件被手工篡改）；`effective_ai()` 改为「DB 优先 → 环境变量回退」；`ai_latency` 文案与错误信息同步指向设置页。
2. `src/db.rs`：`#[cfg(test)]` 追加 `ai_config_roundtrip_and_key_semantics` —— 覆盖保存/回读、`api_key=None` 保留原密钥、`Some("")` 显式清除三段语义（单测内直接构造最小 `Db`，不依赖内容全量同步）。
3. `frontend/src/views/AiSettingsView.vue`：由只读页改造为可编辑表单（backend 下拉 / base_url / model / 密钥密码框 / 清除密钥勾选），POST `/api/ai/config` 保存后回读刷新；来源徽标支持 `database` / `environment` / `none`；密钥输入框占位符提示「留空保持不变」；保留测试连接与安全说明。
4. `frontend/src/App.vue`：导航 ⚙️ 图标 title「AI 代码评价」→「AI 设置」。
5. `README.md`：环境变量一节补充 `RUSTWAY_DATA` 与 AI 配置双通道（设置页 SQLite 优先 / 环境变量回退）说明。
6. `frontend/dist/`：随 `npm run build` 重新生成（该仓库入库构建产物，供 `cargo run` 直接服务）。

### 测试结果

- 静态检查：`cargo fmt --check` ✅ · `cargo clippy --all-targets -- -D warnings` ✅
- 单元测试：`cargo test` ✅ —— 28 单测（上轮 27，+1 ai_config）+ 7 集成 = 35 项全绿，零回归
- 构建验证：`npm run build` ✅（68 模块；chunk 体积警告为既有提示，非本轮引入）
- 冒烟测试（临时数据目录 + 18080 端口，已清理）：10 项全部符合预期 ——
  1. 初始 GET `/api/ai/config` → `source:"none"` ✅
  2. POST 保存 ollama（全默认）→ `saved:true` ✅
  3. 回读 → `source:"database"`，默认地址/模型正确 ✅
  4. POST `/api/ai/latency` → **使用 DB 配置发起真实连接**（修复前此处返回 400「尚未配置」）✅
  5. 私网 base_url（192.168.1.10）→ 400 拒绝（SSRF 防护不回退）✅
  6. 非法 backend → 400 拒绝 ✅
  7. `/` 与 `/assets/*` 均 200 ✅
  8. 保存 openai + 密钥 → `apiKeySet:true` ✅
  9. 回读永不回传密钥明文（只有布尔状态）✅
  10. 不带 api_key 重保存 → 密钥保留（`apiKeySet` 仍 true）✅

### 提交

- 提交信息：`feat(ai): AI 配置持久化闭环——设置页可编辑保存，DB 优先/环境变量回退生效`
- 变更文件：`src/main.rs` · `src/db.rs` · `frontend/src/views/AiSettingsView.vue` · `frontend/src/App.vue` · `README.md` · `frontend/dist/*`（构建产物）· `ITERATION_LOG.md`（新增）

### 下轮建议

- API 指标埋点（请求计数/耗时，维度 5）
- 主观题批改结果的缓存与复习队列联动（README 路线图「间隔重复」）
- `frontend/dist` 改为 CI 构建产物不入库（维度 1/7，需迁移方案评估）
