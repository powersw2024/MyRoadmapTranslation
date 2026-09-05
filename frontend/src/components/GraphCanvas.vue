<script setup>
/**
 * 知识图谱 —— sigma.js (WebGL) + graphology + ForceAtlas2。
 * 物理布局在 Worker 线程计算，GPU 渲染，拖拽/缩放/悬停流畅。
 */
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue'
import Graph from 'graphology'
import Sigma from 'sigma'
import { NodeBorderProgram } from '@sigma/node-border'
import { useManifest, useProgress, useTheme, go } from '../composables/store'

const { manifest } = useManifest()
const { progress } = useProgress()
const { theme } = useTheme()

const container = ref(null)
const wrap = ref(null) // .graph-wrap 根元素（滚轮边界放行监听用）
const tip = ref({ show: false, x: 0, y: 0, title: '', sub: '' })
const hiddenModules = ref(new Set())
// 两类关系的显示开关：前置（手工声明）与相似（算法发现）
const showPrereqs = ref(true)
const showSimilar = ref(true)
// 复习模式：只显示未掌握的知识点
const onlyUnmastered = ref(false)
// 帧预算守卫：连续掉帧时逐级降级（0 正常 → 1 隐藏标签 → 2 隐藏边），恢复后回升
const lodLevel = ref(0)

let graph = null
let renderer = null
let supervisor = null
let hovered = null
let dragged = null
let lastDragEnd = 0
let layoutTimer = null
let rafId = 0
let lastFrame = performance.now()
let slowFrames = 0
let fastFrames = 0
const FRAME_BUDGET = 1000 / 30 // 目标 ≥30fps
// 自转状态：0.06 rad/s ≈ 105s 一圈；交互后暂停 6s
let spinPausedUntil = 0
let lastSpinT = 0

function frameGuard(now) {
  // 先续帧再执行逻辑：任何 early-return 分支都不会杀死循环
  // （此前宽限期 return 未续帧，导致循环第一帧即死、自转与降级守卫全部失效）
  rafId = requestAnimationFrame(frameGuard)
  const delta = now - lastFrame
  lastFrame = now

  // 启动后 5s 宽限期：入场动画/着色器编译的首帧波动不计入降级
  if (frameGuard._mount && now - frameGuard._mount < 5000) return
  if (!frameGuard._mount) frameGuard._mount = now

  // 地球式缓慢自转：绕视图中心匀速旋转；指针交互后暂停数秒
  // ?static=1 供无 GPU/自动化截图环境禁用动画
  const staticMode = new URLSearchParams(window.location.search).has('static')
  if (renderer && !REDUCED_MOTION && !staticMode && now > spinPausedUntil && !dragged) {
    const dt = lastSpinT ? Math.min((now - lastSpinT) / 1000, 0.1) : 0
    if (dt > 0) {
      const cam = renderer.getCamera()
      cam.setState({ angle: cam.angle + 0.06 * dt })
    }
  }
  lastSpinT = now

  if (delta > FRAME_BUDGET * 2) {
    slowFrames++
    fastFrames = 0
    if (slowFrames > 45 && lodLevel.value < 2) {
      lodLevel.value++
      slowFrames = 0
      renderer?.refresh()
      console.warn(`[rustway] 帧预算超限（${delta.toFixed(1)}ms），降级至 LOD ${lodLevel.value}`)
    }
  } else {
    fastFrames++
    if (fastFrames > 120 && lodLevel.value > 0) {
      lodLevel.value--
      fastFrames = 0
      renderer?.refresh()
    }
  }
}

const offModules = computed(() => hiddenModules.value)

// ---------- 动效引擎（canvas 内节点无法用 CSS 动画，全部在 reducer 层实现） ----------
// 入场 stagger 弹出 / 悬停平滑放大 / 掌握光环呼吸 / 点亮闪光；空闲时 rAF 自动停。
const REDUCED_MOTION =
  typeof window.matchMedia === 'function' &&
  window.matchMedia('(prefers-reduced-motion: reduce)').matches

const motion = {
  appearStart: 0, // 入场起始时间戳（0 = 未启用/已完成）
  appearDur: 520, // 单节点弹出时长
  stagger: 3,     // 每节点延迟 ms
  hoverNode: null,
  hoverP: 0,      // 悬停放大进度 0..1
  hoverDir: 0,    // 1 进入 / -1 离开
  flash: new Map(), // node id -> 点亮时间戳
  running: false,
}

const clamp01 = x => (x < 0 ? 0 : x > 1 ? 1 : x)
// 回弹缓动：节点"弹出"的弹性感
const easeOutBack = x => {
  const c1 = 1.70158, c3 = c1 + 1
  return 1 + c3 * Math.pow(x - 1, 3) + c1 * Math.pow(x - 1, 2)
}
const easeOutCubic = x => 1 - Math.pow(1 - x, 3)

function startMotion() {
  if (motion.running || REDUCED_MOTION || !renderer) return
  motion.running = true
  let lastRefresh = 0
  const tick = t => {
    if (!renderer || !graph) { motion.running = false; return }
    const now = performance.now()

    // 悬停 tween（140ms 单程）
    if (motion.hoverDir !== 0) {
      motion.hoverP = clamp01(motion.hoverP + (motion.hoverDir * (t - (tick._pt || t)) / 140))
      if (motion.hoverP === 1 && motion.hoverDir > 0) motion.hoverDir = 0
      if (motion.hoverP === 0 && motion.hoverDir < 0) motion.hoverDir = 0
    }
    tick._pt = t

    // 入场是否仍在进行
    const maxDelay = graph.order * motion.stagger
    const appearing = motion.appearStart > 0 && now < motion.appearStart + motion.appearDur + maxDelay

    // 掌握点亮的闪光过期清理
    for (const [id, ts] of motion.flash) if (now - ts > 700) motion.flash.delete(id)

    // 呼吸/闪光需要持续重绘；限 ~24fps 省电（悬停/入场 tween 期间每帧刷）
    const needsContinuous = motion.hoverDir !== 0 || appearing || motion.flash.size > 0
    const masteredCount = Object.keys(progress.value.mastered).length
    const breathe = masteredCount > 0 || motion.flash.size > 0
    if (needsContinuous || (breathe && t - lastRefresh > 42)) {
      lastRefresh = t
      renderer.refresh()
    }

    const busy = needsContinuous || breathe
    if (busy) requestAnimationFrame(tick)
    else motion.running = false
  }
  requestAnimationFrame(tick)
}

// 掌握状态 diff → 新点亮的节点触发闪光
let prevMastered = {}
function watchMasteredFlashes() {
  const cur = progress.value.mastered
  for (const id of Object.keys(cur)) {
    if (!prevMastered[id] && graph?.hasNode(id)) {
      motion.flash.set(id, performance.now())
    }
  }
  prevMastered = { ...cur }
  startMotion()
}

// ---------- 工具 ----------
function cssVar(name, fallback) {
  const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim()
  return v || fallback
}
function withAlpha(hex, alpha) {
  // #rrggbb -> #rrggbbaa
  if (!hex.startsWith('#') || hex.length < 7) return hex
  const a = Math.round(alpha * 255)
    .toString(16)
    .padStart(2, '0')
  return hex + a
}

// ---------- 建图 ----------
function buildGraph() {
  if (!manifest.value) return
  graph = new Graph({ multi: false, type: 'directed' })

  const mods = manifest.value.modules
  let enterIdx = 0
  for (const m of mods) {
    for (const c of m.chapters) {
      for (const k of c.kps) {
        // 星系布局由后端计算（layout.rs）：x/y 轨道坐标直接消费，前端零布局计算
        graph.addNode(k.id, {
          label: k.title,
          moduleId: m.id,
          moduleTitle: m.title,
          difficulty: k.difficulty,
          orbit: k.orbit ?? 0,
          size: 7, // 星球尺寸统一，核心度只影响轨道层级
          enter: enterIdx++,
          x: k.x ?? 0,
          y: k.y ?? 0,
        })
      }
    }
  }
  for (const [from, to] of manifest.value.edges) {
    if (graph.hasNode(from) && graph.hasNode(to) && !graph.hasEdge(from, to)) {
      graph.addEdge(from, to, { size: 0.35, kind: 'prereq', hidden: false })
    }
  }
  // 相似算法边：更细更淡，与手工前置在视觉上明确区分
  for (const [a, b, score] of manifest.value.similarEdges || []) {
    if (graph.hasNode(a) && graph.hasNode(b) && !graph.hasEdge(a, b)) {
      graph.addEdge(a, b, { size: 0.22, kind: 'similar', score, hidden: false })
    }
  }
}

// ---------- 布局 ----------
// 星系布局已由后端（layout.rs）计算并随 manifest 下发；前端零布局计算。

// ---------- 渲染器 ----------
function createRenderer() {
  renderer = new Sigma(graph, container.value, {
    allowInvalidContainer: true,
    minCameraRatio: 0.12,
    maxCameraRatio: 6,
    nodeProgramClasses: { bordered: NodeBorderProgram },
    defaultNodeType: 'bordered',
    labelDensity: 1.0,
    labelGridCellSize: 90,
    labelRenderedSizeThreshold: 9,
    labelFont: "'PingFang SC','Hiragino Sans GB','Microsoft YaHei',sans-serif",
    labelWeight: '600',
    labelSize: 12,
    nodeReducer: (node, data) => {
      const res = { ...data }
      const now = performance.now()
      const mastered = progress.value.mastered[node]
      const read = progress.value.read[node]

      // 统一尺寸因子：入场弹出 × 悬停放大 × 呼吸 × 闪光
      let scale = 1

      // 节点/线分离：所有节点带背景色描边（在边上"挖出"间隙），默认更大更实
      res.type = 'bordered'
      res.borderColor = cssVar('--bg', '#0a0e1a')
      res.borderSize = 1.6

      // 入场：按序 stagger 弹出（easeOutBack 回弹）
      if (motion.appearStart > 0 && !REDUCED_MOTION) {
        const delay = (data.enter || 0) * motion.stagger
        const p = clamp01((now - motion.appearStart - delay) / motion.appearDur)
        if (p <= 0) {
          res.hidden = true
          return res
        }
        if (p < 1) scale *= Math.max(easeOutBack(p), 0.001)
      }

      // 悬停：tween 进度驱动的平滑放大
      if (motion.hoverP > 0 && (hovered === node || dragged === node || motion.hoverNode === node)) {
        scale *= 1 + 0.45 * easeOutCubic(motion.hoverP)
      }

      if (mastered) {
        // 掌握：金色实心 + 呼吸边（全站唯一的高亮色）
        res.type = 'bordered'
        res.color = '#fbbf24'
        res.borderColor = '#f59e0b'
        res.borderSize = 2.3 + 0.7 * (0.5 + 0.5 * Math.sin(now / 420))
        res.forceLabel = true
        scale *= 1 + 0.05 * Math.sin(now / 420)
      } else {
        // 未掌握/学习中：统一中性色，靠透明度区分状态
        const neutral = cssVar('--node', '#64748b')
        res.color = read ? withAlpha(neutral, 0.95) : withAlpha(neutral, 0.62)
      }

      // 点亮闪光：先膨胀后回落的单次脉冲
      const flashTs = motion.flash.get(node)
      if (flashTs !== undefined) {
        const fp = clamp01((now - flashTs) / 700)
        scale *= 1 + 0.9 * Math.sin(fp * Math.PI)
      }

      res.size = data.size * scale
      if (hovered === node) res.zIndex = 1
      if (dragged === node) res.forceLabel = true
      if (hiddenModules.value.has(data.moduleId)) res.hidden = true
      if (onlyUnmastered.value && mastered) res.hidden = true
      // LOD 降级：1 级隐藏非活跃标签（forceLabel 优先级更高，悬停/掌握仍显示）
      if (lodLevel.value >= 1 && !res.forceLabel && !mastered) res.forceLabel = false
      return res
    },
    edgeReducer: (edge, data) => {
      const res = { ...data }
      const kind = data.kind || 'prereq'
      // 前置 = 实线灰；相似 = 更淡的灰（同一色系，透明度区分层级）
      res.color =
        kind === 'similar'
          ? withAlpha(cssVar('--edge', '#33436b'), 0.5)
          : cssVar('--edge', '#33436b')
      if (hovered && (graph.source(edge) === hovered || graph.target(edge) === hovered)) {
        res.color = cssVar('--accent', '#f97316')
        res.size = kind === 'similar' ? 1.1 : 1.6
        res.zIndex = 1
      }
      if (kind === 'prereq' && !showPrereqs.value) res.hidden = true
      if (kind === 'similar' && !showSimilar.value) res.hidden = true
      if (
        hiddenModules.value.has(graph.getNodeAttribute(graph.source(edge), 'moduleId')) ||
        hiddenModules.value.has(graph.getNodeAttribute(graph.target(edge), 'moduleId'))
      ) {
        res.hidden = true
      }
      // LOD 2：隐藏全部非高亮边（填充率优先给节点）
      if (lodLevel.value >= 2 && res.zIndex !== 1) res.hidden = true
      return res
    },
  })

  // 悬停 → 提示框 + 邻接高亮 + 平滑放大 tween
  renderer.on('enterNode', e => {
    hovered = e.node
    motion.hoverNode = e.node
    motion.hoverDir = 1
    startMotion()
    renderer.refresh({ skipIndexation: false })
    tip.value = {
      show: true,
      x: e.event.original.clientX + 14,
      y: e.event.original.clientY + 12,
      title: graph.getNodeAttribute(e.node, 'label'),
      sub: `${graph.getNodeAttribute(e.node, 'moduleTitle')} · ${masteryText(e.node)} · 难度 ${'★'.repeat(graph.getNodeAttribute(e.node, 'difficulty'))}`,
    }
  })
  renderer.on('leaveNode', () => {
    hovered = null
    motion.hoverDir = -1
    tip.value.show = false
    startMotion()
    renderer.refresh()
  })

  // 点击跳转（拖拽后 200ms 内不触发，避免误跳）
  renderer.on('clickNode', e => {
    if (Date.now() - lastDragEnd < 200) return
    go('/kp/' + e.node)
  })
  renderer.on('clickStage', () => {
    if (!dragMoved && !dragged) tip.value.show = false
  })

  // 节点拖拽
  renderer.on('downNode', e => {
    dragged = e.node
    dragMoved = false
    renderer.getCamera().disable()
    graph.setNodeAttribute(e.node, 'highlighted', true)
  })
  renderer.on('mousemovebody', e => {
    if (!dragged) return
    const pos = renderer.viewportToGraph(e)
    dragMoved = true
    graph.setNodeAttribute(dragged, 'x', pos.x)
    graph.setNodeAttribute(dragged, 'y', pos.y)
  })
  const release = () => {
    if (!dragged) return
    graph.setNodeAttribute(dragged, 'highlighted', false)
    dragged = null
    lastDragEnd = Date.now()
    renderer.getCamera().enable()
    // 布局为后端计算的固定星系坐标，拖拽松手后不重排（保持轨道结构）
  }
  renderer.on('mouseup', release)
  renderer.on('mouseupbody', release)

  // 双击复位视角
  renderer.on('doubleClick', e => {
    e.preventSigmaDefault()
    renderer.getCamera().animate({ x: 0.5, y: 0.5, ratio: 1, angle: 0 }, { duration: 400 })
  })
}

function masteryText(id) {
  if (progress.value.mastered[id]) return '已掌握 ✓'
  if (progress.value.read[id]) return '学习中…'
  return '未开始'
}

// ---------- 模块可见性 ----------
function toggleModule(id) {
  const s = new Set(hiddenModules.value)
  s.has(id) ? s.delete(id) : s.add(id)
  hiddenModules.value = s
  graph.forEachNode((node, attr) => {
    if (attr.moduleId === id) graph.setNodeAttribute(node, 'hidden', s.has(id))
  })
  renderer.refresh()
  recountVisible()
}

const similarCount = computed(() => (manifest.value?.similarEdges || []).length)
function toggleRelations(kind) {
  if (kind === 'prereq') showPrereqs.value = !showPrereqs.value
  else showSimilar.value = !showSimilar.value
  renderer?.refresh()
}
function toggleOnlyUnmastered() {
  onlyUnmastered.value = !onlyUnmastered.value
  renderer?.refresh()
  recountVisible()
}

const visibleCount = ref(0)
function recountVisible() {
  if (!graph) { visibleCount.value = 0; return }
  let n = 0
  graph.forEachNode((_, attr) => { if (!attr.hidden) n++ })
  visibleCount.value = n
}

// ---------- 生命周期 ----------
onMounted(async () => {
  if (!manifest.value) await new Promise(r => watch(manifest, r, { once: true }))
  buildGraph()
  createRenderer()
  // 星系全景：初始视野覆盖全部轨道（外圈尘埃带半径 ~640）
  renderer.getCamera().setState({ x: 0.5, y: 0.5, ratio: 1.45, angle: 0 })
  renderer.refresh()
  recountVisible()
  // 入场动画：布局落定后按序弹出
  prevMastered = { ...progress.value.mastered }
  if (!REDUCED_MOTION) motion.appearStart = performance.now() + 80
  startMotion()
  lastFrame = performance.now()
  rafId = requestAnimationFrame(frameGuard)

  // 滚轮分区：仅悬停图谱时缩放（sigma 容器天然如此）；
  // 缩放到达边界后继续滚动 → 放行页面滚动，避免"卡"在图谱上
  const el = wrap.value
  el.addEventListener(
    'wheel',
    e => {
      if (!renderer) return
      const cam = renderer.getCamera()
      const atZoomOutLimit = cam.ratio <= 0.12 * 1.1 && e.deltaY > 0
      const atZoomInLimit = cam.ratio >= 6 * 0.95 && e.deltaY < 0
      if (atZoomOutLimit || atZoomInLimit) {
        // 阻止 sigma 收到事件（不 preventDefault → 浏览器滚动页面）
        e.stopImmediatePropagation()
      }
    },
    true
  )
  // 指针交互期间暂停自转，松手数秒后恢复
  const pauseSpin = () => {
    spinPausedUntil = performance.now() + 6000
  }
  el.addEventListener('pointerdown', pauseSpin)
  el.addEventListener('pointerup', pauseSpin)
  el.addEventListener('wheel', pauseSpin, { passive: true })
  onBeforeUnmount(() => {
    el.removeEventListener('pointerdown', pauseSpin)
    el.removeEventListener('pointerup', pauseSpin)
    el.removeEventListener('wheel', pauseSpin)
  })
})

onBeforeUnmount(() => {
  cancelAnimationFrame(rafId)
  if (renderer) { renderer.kill(); renderer = null }
})

// 进度变化 → 闪光检测 + 重绘；主题变化 → 重绘（读取最新颜色）
watch(progress, () => {
  watchMasteredFlashes()
  renderer?.refresh()
}, { deep: true })
watch(theme, () => renderer?.refresh())

const legend = computed(() => {
  if (!manifest.value) return []
  return manifest.value.modules.map(m => {
    let total = 0, done = 0
    for (const c of m.chapters) {
      for (const k of c.kps) {
        total++
        if (progress.value.mastered[k.id]) done++
      }
    }
    return { id: m.id, title: m.title, icon: m.icon, total, done }
  })
})
</script>

<template>
  <div ref="wrap" class="graph-wrap">
    <div ref="container" class="sigma-container" />

    <div class="graph-legend">
      <div class="t-caption c-faint" style="font-weight: 700">知识面图例（点击隐藏）</div>
      <div
        v-for="m in legend"
        :key="m.id"
        class="legend-item"
        :class="{ off: hiddenModules.has(m.id) }"
        style="color: var(--accent)"
        @click="toggleModule(m.id)"
      >
        <span>{{ m.icon }}</span>
        <span class="c-text" style="flex: 1">{{ m.title }}</span>
        <span class="legend-bar"><div :style="{ width: m.total ? (m.done / m.total) * 100 + '%' : 0 }" /></span>
        <span class="t-caption t-num">{{ m.done }}/{{ m.total }}</span>
      </div>
      <div class="legend-divider" />
      <div class="legend-item" :class="{ off: !showPrereqs }" style="color: var(--muted)" @click="toggleRelations('prereq')">
        <span class="c-muted">—</span>
        <span class="c-text" style="flex: 1">前置依赖（手工）</span>
      </div>
      <div class="legend-item" :class="{ off: !showSimilar }" style="color: var(--muted)" @click="toggleRelations('similar')">
        <span class="c-faint">┈</span>
        <span class="c-text" style="flex: 1">相似关联（算法）</span>
      </div>
      <div class="legend-item" :class="{ off: !onlyUnmastered }" style="color: var(--muted)" @click="toggleOnlyUnmastered">
        <span style="color: var(--gold)">◎</span>
        <span class="c-text" style="flex: 1">只看未掌握</span>
      </div>
    </div>

    <div class="graph-stats">
      {{ visibleCount }} 个知识点 · {{ graph ? graph.size : 0 }} 条关联 · 拖动节点 / 滚轮缩放 / 双击复位
      <span v-if="lodLevel > 0" class="c-gold"> · 性能模式 L{{ lodLevel }}</span>
    </div>

    <Teleport to="body">
      <div v-show="tip.show" class="tooltip" :style="{ left: tip.x + 'px', top: tip.y + 'px' }">
        <div class="t-title">{{ tip.title }}</div>
        <div class="t-sub">{{ tip.sub }}</div>
      </div>
    </Teleport>
  </div>
</template>
