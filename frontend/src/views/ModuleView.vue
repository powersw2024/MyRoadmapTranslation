<script setup>
import { ref, computed } from 'vue'
import { useManifest, useProgress, go } from '../composables/store'
import QuizWidget from '../components/QuizWidget.vue'

const props = defineProps({ id: { type: String, required: true } })
const { manifest } = useManifest()
const { progress } = useProgress()
const openChapters = ref(new Set())

const module = computed(() => manifest.value?.modules.find(m => m.id === props.id))

function isOpen(cid) {
  return openChapters.value.has(cid)
}
function toggle(cid) {
  const s = new Set(openChapters.value)
  s.has(cid) ? s.delete(cid) : s.add(cid)
  openChapters.value = s
}
function kpClass(k) {
  return progress.value.mastered[k.id] ? 'done' : ''
}
function dotColor(m, k) {
  return progress.value.mastered[k.id] ? 'var(--ok)' : m.color
}
</script>

<template>
  <div v-if="module">
    <div class="breadcrumb stagger">
      <a href="#/">首页</a><span class="sep">/</span><span>{{ module.title }}</span>
    </div>

    <div class="card page-head pad-lg stagger" :style="{ borderTop: '3px solid ' + module.color }">
      <div class="meta-row">
        <span class="chip"><span class="dot" :style="{ background: module.color }" />模块 {{ module.num }}</span>
        <span class="chip">{{ module.chapters.length }} 章</span>
        <span class="chip">{{ module.chapters.reduce((s, c) => s + c.kps.length, 0) }} 个知识点</span>
      </div>
      <h1 class="t-page">{{ module.icon }} {{ module.title }}</h1>
      <p class="summary t-body c-muted">{{ module.subtitle }}</p>
    </div>

    <div
      v-for="(c, ci) in module.chapters"
      :key="c.id"
      class="card chapter-item stagger"
      :style="{ animationDelay: ci * 60 + 'ms' }"
    >
      <div class="chapter-head" :class="{ open: isOpen(c.id) }" @click="toggle(c.id)">
        <span class="c-num" :style="{ background: module.color }">{{ module.num }}.{{ ci + 1 }}</span>
        <span class="c-title t-item">{{ c.title }}</span>
        <span class="t-caption c-faint">{{ c.kps.length }} 个知识点</span>
        <span class="arrow">▶</span>
      </div>
      <div class="chapter-body" :class="{ open: isOpen(c.id) }">
        <div>
          <div class="chapter-inner">
            <div class="obj-list">
              <div class="section-title">🎯 学习目标</div>
              <ul class="c-muted" style="padding-left: 20px; line-height: 1.9">
                <li v-for="(o, i) in c.objectives" :key="i" class="t-note">{{ o }}</li>
              </ul>
            </div>
            <div class="section-title">🧩 知识点</div>
            <div class="kp-chips">
              <span
                v-for="k in c.kps"
                :key="k.id"
                class="kp-chip"
                :class="kpClass(k)"
                @click="go('/kp/' + k.id)"
              >
                <span class="dot" :style="{ background: dotColor(module, k) }" />
                {{ k.title }}
                <span v-if="progress.mastered[k.id]" class="star">✓</span>
                <span v-if="k.hasDetail" class="c-faint" style="font-size: 10px">📝</span>
              </span>
            </div>
            <div class="section-title">🏁 关卡测验（覆盖本章全部知识点）</div>
            <QuizWidget :questions="c.checkpoint" />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
