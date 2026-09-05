<script setup>
import { ref, computed } from 'vue'
import { useManifest, useProgress, useDelayedFlag, go } from '../composables/store'
import GraphCanvas from '../components/GraphCanvas.vue'
import CountUp from '../components/CountUp.vue'

const { manifest } = useManifest()
const { progress } = useProgress()
const barsGo = useDelayedFlag(120)

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

const stats = computed(() => {
  if (!manifest.value) return []
  const s = manifest.value.stats
  return [
    { icon: '📦', label: '模块', value: s.modules },
    { icon: '📚', label: '章节', value: s.chapters },
    { icon: '🧩', label: '知识点', value: s.kps },
    { icon: '📝', label: '测验题', value: s.quiz },
    { icon: '⏱️', label: '预计学习时长', value: Math.round(s.minutes / 60), suffix: 'h+' },
  ]
})

// 知识入口分类（来自 curriculum.json 的 module.category，可任意扩展）
const activeCategory = ref('全部')
const categories = computed(() => {
  const set = new Set((manifest.value?.modules || []).map(m => m.category || '通用'))
  return ['全部', ...set]
})
const filteredModules = computed(() => {
  if (!manifest.value) return []
  return manifest.value.modules.filter(
    m => activeCategory.value === '全部' || (m.category || '通用') === activeCategory.value
  )
})
</script>

<template>
  <div v-if="manifest">
    <!-- Hero -->
    <div class="stagger" style="animation-delay: 0ms; margin-bottom: 16px">
      <h1 class="hero-title">{{ manifest.title }}</h1>
      <p class="hero-sub">{{ manifest.subtitle }}</p>
    </div>

    <!-- 统计（数字滚动动效） -->
    <div class="stats-row stagger" style="animation-delay: 60ms">
      <div v-for="(st, i) in stats" :key="st.label" class="card stat-card">
        <span class="icon">{{ st.icon }}</span>
        <span class="num num-pop t-num" :style="{ animationDelay: 120 + i * 70 + 'ms' }">
          <CountUp :value="st.value" />{{ st.suffix || '' }}
        </span>
        <span class="label">{{ st.label }}</span>
      </div>
    </div>

    <!-- 知识图谱（主角） -->
    <GraphCanvas />

    <!-- 知识入口（按分类过滤） -->
    <div class="section-title stagger" style="animation-delay: 240ms; margin-top: 28px">📚 知识入口</div>
    <div class="category-tabs stagger" style="animation-delay: 260ms">
      <span
        v-for="cat in categories"
        :key="cat"
        class="chip cat-tab"
        :class="{ active: activeCategory === cat }"
        @click="activeCategory = cat"
      >
        {{ cat }}
      </span>
    </div>
    <div class="module-grid">
      <div
        v-for="(m, i) in filteredModules"
        :key="m.id"
        class="card card-hover module-card stagger"
        :style="{ animationDelay: 260 + i * 50 + 'ms' }"
        @click="go('/module/' + m.id)"
      >
        <div class="m-head">
          <span class="m-icon">{{ m.icon }}</span>
          <span class="m-title">{{ m.num }}. {{ m.title }}</span>
        </div>
        <div class="m-sub">{{ m.subtitle }}</div>
        <div class="m-progress">
          <div
            class="grow-bar"
            :class="{ go: barsGo }"
            :style="{ '--bar-w': (moduleProgress[m.id]?.pct || 0) + '%' }"
          />
        </div>
        <div class="m-meta">
          <span>{{ m.chapters.length }} 章</span>
          <span>{{ m.chapters.reduce((s, c) => s + c.kps.length, 0) }} 个知识点</span>
          <span v-if="moduleProgress[m.id]?.done" class="c-ok">已掌握 {{ moduleProgress[m.id].done }}</span>
          <span class="c-accent" style="margin-left: auto">进入 →</span>
        </div>
      </div>
    </div>
  </div>
</template>
