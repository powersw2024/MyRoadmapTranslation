<script setup>
import { computed } from 'vue'
import { useRoute, useManifest, useProgress, useTheme, go } from './composables/store'
import ThemeToggle from './components/ThemeToggle.vue'
import GlobalSearch from './components/GlobalSearch.vue'
import HomeView from './views/HomeView.vue'
import KpView from './views/KpView.vue'
import ModuleView from './views/ModuleView.vue'
import AiSettingsView from './views/AiSettingsView.vue'

const route = useRoute()
const { manifest, manifestError } = useManifest()
const { progress, masteredCount } = useProgress()
const { theme } = useTheme()

const view = computed(() => {
  if (route.value.name === 'kp') return KpView
  if (route.value.name === 'module') return ModuleView
  if (route.value.name === 'ai') return AiSettingsView
  return HomeView
})

const totalPlaced = computed(() => {
  if (!manifest.value) return 0
  let n = 0
  for (const m of manifest.value.modules) for (const c of m.chapters) n += c.kps.length
  return n
})
</script>

<template>
  <header class="topbar">
    <a class="brand" href="#/" @click.prevent="go('/')">
      <span class="brand-icon">🦀</span>
      <span class="brand-text">
        <strong>RustWay</strong>
        <small>知识学习之路</small>
      </span>
    </a>

    <GlobalSearch />

    <div v-if="manifest" class="progress-pill" title="已掌握 / 已编排知识点">
      <span class="progress-num">{{ masteredCount }}/{{ totalPlaced }}</span>
      <div class="progress-bar">
        <div class="progress-fill" :style="{ width: totalPlaced ? (masteredCount / totalPlaced) * 100 + '%' : '0%' }" />
      </div>
    </div>

    <a class="icon-link" href="#/ai" title="AI 设置" @click.prevent="go('/ai')">⚙️</a>

    <ThemeToggle />
  </header>

  <main class="container">
    <div v-if="manifestError" class="card info-card c-bad">{{ manifestError }}</div>

    <!-- 骨架屏：数据加载中的结构占位，避免空白闪烁 -->
    <div v-else-if="!manifest" class="skeleton-page" aria-label="加载中">
      <div class="skel skel-title" />
      <div class="skel skel-line" style="width: 60%" />
      <div class="stats-row">
        <div v-for="i in 5" :key="i" class="card stat-card skel-card" />
      </div>
      <div class="card graph-wrap skel-graph" />
      <div class="home-grid">
        <div class="card skel-card tall" />
        <div class="card skel-card tall" />
      </div>
    </div>

    <Transition name="page" mode="out-in">
      <component
        :is="view"
        v-if="manifest"
        :key="route.name + (route.id || '')"
        :id="route.id"
      />
    </Transition>
  </main>

  <footer class="footer">
    <span>RustWay v{{ manifest?.version || '…' }} — 通用学习框架 · 内容与代码分离 · 新增知识点无需改代码</span>
    <a href="https://github.com/powersw2024/MyRoadmapTranslation" target="_blank" rel="noopener">GitHub ↗</a>
    <span class="c-faint">当前主题：{{ theme === 'dark' ? '暗色' : '亮色' }}</span>
  </footer>
</template>
