<script setup>
import { ref, computed, watch } from 'vue'
import { useManifest, go } from '../composables/store'

const { manifest } = useManifest()
const q = ref('')
const open = ref(false)
const activeIdx = ref(0)

const allKps = computed(() => {
  if (!manifest.value) return []
  const list = []
  for (const m of manifest.value.modules) {
    for (const c of m.chapters) {
      for (const k of c.kps) list.push({ ...k, moduleName: m.title, moduleColor: m.color })
    }
  }
  for (const k of manifest.value.unplaced) list.push({ ...k, moduleName: '待编排', moduleColor: '#64748b' })
  return list
})

const results = computed(() => {
  const query = q.value.trim().toLowerCase()
  if (!query) return []
  return allKps.value
    .filter(
      k =>
        k.title.toLowerCase().includes(query) ||
        k.summary.toLowerCase().includes(query) ||
        (k.tags || []).some(t => t.toLowerCase().includes(query))
    )
    .slice(0, 8)
})

watch(q, () => {
  activeIdx.value = 0
  open.value = true
})

function pick(k) {
  open.value = false
  q.value = ''
  go('/kp/' + k.id)
}

function onKey(e) {
  if (e.key === 'ArrowDown') {
    activeIdx.value = Math.min(activeIdx.value + 1, results.value.length - 1)
    e.preventDefault()
  } else if (e.key === 'ArrowUp') {
    activeIdx.value = Math.max(activeIdx.value - 1, 0)
    e.preventDefault()
  } else if (e.key === 'Enter' && results.value[activeIdx.value]) {
    pick(results.value[activeIdx.value])
  } else if (e.key === 'Escape') {
    open.value = false
  }
}
</script>

<template>
  <div class="search-wrap" @blur="open = false">
    <input
      v-model="q"
      type="search"
      placeholder="搜索知识点…（如：所有权 / B+ 树 / epoll）"
      @focus="open = true"
      @keydown="onKey"
    />
    <div v-if="open && results.length" class="search-results" @mousedown.prevent>
      <div
        v-for="(k, i) in results"
        :key="k.id"
        class="search-item"
        :class="{ active: i === activeIdx }"
        @click="pick(k)"
      >
        <span class="dot" :style="{ background: k.moduleColor }" />
        <span>{{ k.title }}</span>
        <span class="t-caption c-faint" style="margin-left: auto">{{ k.moduleName }}</span>
      </div>
    </div>
  </div>
</template>
