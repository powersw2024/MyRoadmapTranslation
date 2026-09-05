# ADR-0001 · 知识图谱渲染架构决策

- 状态：已接受
- 日期：2026-09-05
- 关联：RustWay 主页知识图谱

## 背景

RustWay 主页以知识图谱展示学习成果（知识点 = 节点，前置依赖 = 边）。需要决策渲染技术栈与扩容路径。

当前事实规模：

| 指标 | 值 |
| --- | --- |
| 知识点（节点） | ~200（上限设计目标 5 万） |
| 前置关系（边） | ~230 |
| 交付形态 | 本地 Rust 服务 + 浏览器（localhost，无公网部署要求） |

## 决策

**当前采用 sigma.js（WebGL 渲染）+ graphology + ForceAtlas2**，配帧预算守卫与自动降级。

理由：

1. 规模匹配：sigma.js 的 WebGL 实例化渲染在数万节点内单 draw call 量级流畅，当前规模有 2 个数量级余量；
2. 图领域专精：内置 GPU picking（悬停/点击检测不走 CPU 遍历）、缩放/拖拽相机、标签 LOD（labelRenderedSizeThreshold / labelDensity）；
3. 布局成本可控：节点 ≤ 5 万时 ForceAtlas2 可在 Worker 中运行；我们进一步用「同步跑有限次迭代 + 归一化 + 运行时只在拖拽后局部重排」把主线程阻塞压到一次 ~50ms；
4. 本地应用无传输瓶颈：全量课程清单 < 1MB，一次 /api/manifest 即可载入内存，无需服务端视口裁剪。

## 明确否决的替代方案（当前规模）

| 方案 | 否决原因 |
| --- | --- |
| D3 + SVG | DOM 节点 O(n) 操作，万级即卡 |
| ECharts Graph / AntV G6 (Canvas) | 无实例化渲染，5 万节点掉帧 |
| 自研 WebGL2/WebGPU 渲染器 | 工程量大；当前规模收益为零，仅在未来触发阈值后评估 |

## 扩容路径（触发条件 → 行动）

| 触发条件 | 行动 |
| --- | --- |
| 节点 > 5 万，或加载期布局 > 300ms | 布局预计算：Rust 侧离线跑 FA2，产物随内容发布；前端只读坐标 |
| 节点 > 20 万，或 sigma.js 标签/边渲染成为瓶颈 | 切换 deck.gl（WebGL2 实例化，ScatterplotLayer + LineLayer），节点/边数据转 TypedArray 列式 |
| 单清单文件 > 100MB，或解析 > 1s | 内容格式二进制化（Arrow IPC 或 bincode + mmap），Rust 侧编译产物 |
| 目标改为桌面应用 / 数据 > 2GB | Tauri 2：Rust 进程 mmap + IPC 流式喂给 WebView；渲染层不变 |
| 需要百万级 | Rust→WASM（wasm-bindgen）做 CSR/QuadTree/picking + regl 或裸 WebGL2 实例化渲染；LOD 分层（社区聚合 → 枢纽节点 → 全量细节），边 LOD 用 top-K 或边捆绑 |

本 ADR 是「决策就绪」记录：触发前不实现，触发时按表行动，避免重新论证。

## 已实现的性能护栏（当前代码）

1. **帧预算守卫**：`GraphCanvas.vue` 持续监测 `requestAnimationFrame` 间隔，连续超预算自动降级（隐藏标签 → 隐藏边 → 提示），恢复后自动回升；
2. **布局不驻留**：初始布局同步跑 170 次迭代 + 离群截断 + 视口归一化；运行时仅拖拽松手后局部重排 1.5s，随后 Worker 停止（零常驻 CPU）；
3. **GPU picking**：悬停/点击由 sigma.js 在 GPU 侧完成，无 CPU 遍历；
4. **增量重绘**：进度/主题变化走 `renderer.refresh()` 的 reducer 路径，不重建图数据。

## 验收对照（本地应用清单适配版）

- [x] 冷启动到首帧图谱可见 < 2s（本地 API + 静态资源）
- [x] 全量数据进内存，无二次请求
- [x] 稳态平移/缩放 60fps（WebGL 实例化）
- [x] picking 无 CPU 遍历
- [x] 运行时无常驻布局计算
- [x] 帧率守卫 + 自动降级
- [ ] （触发阈值后再实现：二进制格式 / deck.gl / WASM / Tauri，见上表）
