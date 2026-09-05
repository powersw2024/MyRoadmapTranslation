<script setup>
/**
 * 知识图谱 —— sigma.js (WebGL) + graphology + ForceAtlas2。
 * 物理布局在 Worker 线程计算，GPU 渲染，拖拽/缩放/悬停流畅。
 */
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue'
import Graph from 'graphology'
import Sigma from 'sigma'
import { NodeBorderProgram } from '@sigma/node-border'
import forceAtlas2 from 'graphology-layout-forceatlas2'
import FA2Supervisor from 'graphology-layout-forceatlas2/worker'
import circular from 'graphology-layout/circular'
import { useManifest, useProgress, useTheme, go } from '../composables/store'

const { manifest } = useManifest()
const { progress } = useProgress()
const { theme } = useTheme()

const container = ref(null)
const tip = ref({ show: false, x: 0, y: 0, title: '', sub: '' })
const hiddenModules = ref(new Set())
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

function frameGuard(now) {
  const delta = now - lastFrame
  lastFrame = now
  if (delta > FRAME_BUDGET * 2) {
    slowFrames++
    fastFrames = 0
    if (slowFrames > 20 && lodLevel.value < 2) {
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
  rafId = requestAnimationFrame(frameGuard)
}

const offModules = computed(() => hiddenModules.value)

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
  for (const m of mods) {
    for (const c of m.chapters) {
      for (const k of c.kps) {
        graph.addNode(k.id, {
          label: k.title,
          moduleId: m.id,
          moduleTitle: m.title,
          color: m.color,
          difficulty: k.difficulty,
          size: 4 + k.difficulty * 2.4,
          x: 0, y: 0,
        })
      }
    }
  }
  for (const [from, to] of manifest.value.edges) {
    if (graph.hasNode(from) && graph.hasNode(to) && !graph.hasEdge(from, to)) {
      graph.addEdge(from, to, { size: 0.7, color: '#8899bb', hidden: false })
    }
  }

  // 初始位置：按模块扇形摆放，FA2 只需微调
  circular.assign(graph)
  const modIndex = Object.fromEntries(mods.map((m, i) => [m.id, i]))
  graph.forEachNode((node, attr) => {
    const i = modIndex[attr.moduleId] ?? 0
    const n = mods.length
    const angle = (i / n) * Math.PI * 2 - Math.PI / 2
    const r = 140 + attr.difficulty * 90
    graph.setNodeAttribute(node, 'x', Math.cos(angle) * r * 1.6 + (Math.random() - 0.5) * 120)
    graph.setNodeAttribute(node, 'y', Math.sin(angle) * r + (Math.random() - 0.5) * 120)
  })
}

// ---------- 布局 ----------
const FA2_SETTINGS = {
  gravity: 2.2,
  scalingRatio: 8,
  barnesHutOptimize: true,
  barnesHutTheta: 0.55,
  slowDown: 12,
  edgeWeightInfluence: 0,
  adjustSizes: true,
}
const VW = 1600, VH = 880, CX = VW / 2, CY = VH / 2

function runLayoutSync(iterations = 170) {
  forceAtlas2.assign(graph, { iterations, settings: FA2_SETTINGS })
  clampOutliers()
  fitToView()
}

// 把飞得太远的节点截断回密度核心区（中位数 ± 4×MAD）
function clampOutliers() {
  for (const axis of ['x', 'y']) {
    const vals = graph.mapNodes((_, a) => a[axis]).sort((a, b) => a - b)
    const median = vals[Math.floor(vals.length / 2)]
    const absDiff = vals.map(v => Math.abs(v - median)).sort((a, b) => a - b)
    const mad = Math.max(absDiff[Math.floor(absDiff.length / 2)], 1)
    const lo = median - 4 * mad, hi = median + 4 * mad
    graph.forEachNode((n, a) => {
      if (a[axis] < lo) graph.setNodeAttribute(n, axis, lo)
      if (a[axis] > hi) graph.setNodeAttribute(n, axis, hi)
    })
  }
}

// 归一化：把布局缩放平移进固定视口框
function fitToView() {
  let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity
  graph.forEachNode((_, a) => {
    if (a.x < minX) minX = a.x
    if (a.x > maxX) maxX = a.x
    if (a.y < minY) minY = a.y
    if (a.y > maxY) maxY = a.y
  })
  const s = Math.min(1250 / Math.max(maxX - minX, 1), 680 / Math.max(maxY - minY, 1))
  const mx = (minX + maxX) / 2, my = (minY + maxY) / 2
  graph.forEachNode((n, a) => {
    graph.setNodeAttribute(n, 'x', CX + (a.x - mx) * s)
    graph.setNodeAttribute(n, 'y', CY + (a.y - my) * s)
  })
}

function startLayout(ms) {
  if (!graph) return
  stopLayout()
  supervisor = new FA2Supervisor(graph, { settings: FA2_SETTINGS })
  supervisor.start()
  if (ms) layoutTimer = setTimeout(stopLayout, ms)
}
function stopLayout() {
  if (layoutTimer) { clearTimeout(layoutTimer); layoutTimer = null }
  if (supervisor) { supervisor.stop(); supervisor.kill(); supervisor = null }
}

// ---------- 渲染器 ----------
function createRenderer() {
  renderer = new Sigma(graph, container.value, {
    allowInvalidContainer: true,
    minCameraRatio: 0.12,
    maxCameraRatio: 6,
    nodeProgramClasses: { bordered: NodeBorderProgram },
    defaultNodeType: 'circle',
    labelDensity: 0.7,
    labelGridCellSize: 90,
    labelRenderedSizeThreshold: 13,
    labelFont: "'PingFang SC','Hiragino Sans GB','Microsoft YaHei',sans-serif",
    labelWeight: '600',
    labelSize: 12,
    nodeReducer: (node, data) => {
      const res = { ...data }
      const mastered = progress.value.mastered[node]
      const read = progress.value.read[node]
      res.type = mastered ? 'bordered' : 'circle'
      if (mastered) {
        res.borderColor = '#fbbf24'
        res.borderSize = 2.4
        res.size = data.size * 1.25
        res.forceLabel = true
      } else {
        res.color = read ? withAlpha(data.color, 0.78) : withAlpha(data.color, 0.42)
      }
      if (hovered === node) {
        res.size = data.size * 1.45
        res.forceLabel = true
        res.zIndex = 1
      }
      if (dragged === node) res.forceLabel = true
      if (hiddenModules.value.has(data.moduleId)) res.hidden = true
      // LOD 降级：1 级隐藏非活跃标签（forceLabel 优先级更高，悬停/掌握仍显示）
      if (lodLevel.value >= 1 && !res.forceLabel && !mastered) res.forceLabel = false
      return res
    },
    edgeReducer: (edge, data) => {
      const res = { ...data }
      res.color = cssVar('--edge', '#33436b')
      if (hovered && (graph.source(edge) === hovered || graph.target(edge) === hovered)) {
        res.color = cssVar('--accent', '#f97316')
        res.size = 1.6
        res.zIndex = 1
      }
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

  // 悬停 → 提示框 + 邻接高亮
  renderer.on('enterNode', e => {
    hovered = e.node
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
    tip.value.show = false
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
    startLayout(1500) // 松手后短暂重排
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
  runLayoutSync(170)
  renderer.refresh()
  recountVisible()
  lastFrame = performance.now()
  rafId = requestAnimationFrame(frameGuard)
})

onBeforeUnmount(() => {
  cancelAnimationFrame(rafId)
  stopLayout()
  if (renderer) { renderer.kill(); renderer = null }
})

// 进度 / 主题变化 → 重绘（读取最新颜色与掌握状态）
watch(progress, () => renderer?.refresh(), { deep: true })
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
    return { id: m.id, title: m.title, icon: m.icon, color: m.color, total, done }
  })
})
</script>

<template>
  <div class="graph-wrap">
    <div ref="container" class="sigma-container" />

    <div class="graph-legend">
      <div style="font-size: 11px; font-weight: 700; color: var(--faint); letter-spacing: 1px">模块图例（点击隐藏）</div>
      <div
        v-for="m in legend"
        :key="m.id"
        class="legend-item"
        :class="{ off: hiddenModules.has(m.id) }"
        :style="{ color: m.color }"
        @click="toggleModule(m.id)"
      >
        <span>{{ m.icon }}</span>
        <span style="color: var(--text); flex: 1">{{ m.title }}</span>
        <span class="legend-bar"><div :style="{ width: m.total ? (m.done / m.total) * 100 + '%' : 0 }" /></span>
        <span style="font-size: 10px">{{ m.done }}/{{ m.total }}</span>
      </div>
    </div>

    <div class="graph-stats">
      {{ visibleCount }} 个知识点 · {{ graph ? graph.order : 0 }} 节点 / {{ graph ? graph.size : 0 }} 边 · 拖动节点 / 滚轮缩放 / 双击复位
      <span v-if="lodLevel > 0" style="color: var(--gold)"> · 性能模式 L{{ lodLevel }}</span>
    </div>

    <Teleport to="body">
      <div v-show="tip.show" class="tooltip" :style="{ left: tip.x + 'px', top: tip.y + 'px' }">
        <div class="t-title">{{ tip.title }}</div>
        <div class="t-sub">{{ tip.sub }}</div>
      </div>
    </Teleport>
  </div>
</template>
