<script setup>
import { computed } from 'vue'
import { useManifest, useProgress, go } from '../composables/store'
import GraphCanvas from '../components/GraphCanvas.vue'

const { manifest } = useManifest()
const { progress } = useProgress()

const moduleProgress = computed(() => {
  const map = {}
  if (!manifest.value) return map
  for (const m of manifest.value.modules) {
    let total = 0, done = 0
    for (const c of m.chapters) {
      for (const k of c.kps) {
        total++
        if (progress.value.mastered[k.id]) done++
      }
    }
    map[m.id] = { total, done, pct: total ? (done / total) * 100 : 0 }
  }
  return map
})
</script>

<template>
  <div v-if="manifest">
    <!-- Hero -->
    <div class="stagger" style="animation-delay: 0ms; margin-bottom: 16px">
      <h1 style="font-size: 26px; margin-bottom: 6px">
        {{ manifest.title }}
      </h1>
      <p style="color: var(--muted); font-size: 14px; line-height: 1.7; max-width: 860px">
        {{ manifest.subtitle }}
      </p>
    </div>

    <!-- 统计 -->
    <div class="stats-row stagger" style="animation-delay: 60ms">
      <div class="card stat-card"><span class="num">{{ manifest.stats.modules }}</span><span class="label">模块</span></div>
      <div class="card stat-card"><span class="num">{{ manifest.stats.chapters }}</span><span class="label">章节</span></div>
      <div class="card stat-card"><span class="num">{{ manifest.stats.kps }}</span><span class="label">知识点</span></div>
      <div class="card stat-card"><span class="num">{{ manifest.stats.quiz }}</span><span class="label">测验题</span></div>
      <div class="card stat-card"><span class="num">{{ Math.round(manifest.stats.minutes / 60) }}h+</span><span class="label">预计学习时长</span></div>
    </div>

    <!-- 知识图谱（主角） -->
    <GraphCanvas />

    <!-- 方法论 + 待编排 -->
    <div class="home-grid">
      <div class="card info-card stagger" style="animation-delay: 120ms">
        <div class="section-title">🧪 科学学习方法（本教程的编排原则）</div>
        <p class="intro-text">{{ manifest.methodology.intro }}</p>
        <ul>
          <li v-for="(r, i) in manifest.methodology.rules" :key="i">{{ r }}</li>
        </ul>
      </div>
      <div class="card info-card stagger" style="animation-delay: 180ms">
        <div class="section-title">📥 待编排知识点（{{ manifest.unplaced.length }}）</div>
        <p class="intro-text">
          这些知识点已通过内容校验，但尚未挂入任何章节——这正是「任意插入」能力的体现：新建 JSON 文件即可入库，随时编入任意章节。
        </p>
        <div class="unplaced-list" v-if="manifest.unplaced.length">
          <span v-for="k in manifest.unplaced" :key="k.id" class="chip" @click="go('/kp/' + k.id)">
            {{ k.title }} · {{ k.minutes }}min
          </span>
        </div>
        <p v-else class="empty-tip">全部知识点都已编入学习路径。</p>
      </div>
    </div>

    <!-- 模块卡片 -->
    <div class="section-title stagger" style="animation-delay: 240ms; margin-top: 28px">📚 学习模块</div>
    <div class="module-grid">
      <div
        v-for="(m, i) in manifest.modules"
        :key="m.id"
        class="card card-hover module-card stagger"
        :style="{ '--mc': m.color, animationDelay: 260 + i * 50 + 'ms' }"
        @click="go('/module/' + m.id)"
      >
        <div class="m-head">
          <span class="m-icon">{{ m.icon }}</span>
          <span class="m-title">{{ m.num }}. {{ m.title }}</span>
        </div>
        <div class="m-sub">{{ m.subtitle }}</div>
        <div class="m-progress">
          <div :style="{ width: (moduleProgress[m.id]?.pct || 0) + '%', background: m.color }" />
        </div>
        <div class="m-meta">
          <span>{{ m.chapters.length }} 章</span>
          <span>{{ m.chapters.reduce((s, c) => s + c.kps.length, 0) }} 个知识点</span>
          <span v-if="moduleProgress[m.id]?.done" style="color: var(--ok)">已掌握 {{ moduleProgress[m.id].done }}</span>
          <span style="margin-left: auto; color: var(--accent)">进入 →</span>
        </div>
      </div>
    </div>
  </div>
</template>
