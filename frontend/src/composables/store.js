import { ref, computed, watch, onMounted } from 'vue'

// ---------- Hash 路由（轻量，无额外依赖） ----------
const hash = ref(window.location.hash)
window.addEventListener('hashchange', () => {
  hash.value = window.location.hash
  window.scrollTo({ top: 0 })
})

export function useRoute() {
  return computed(() => {
    const parts = (hash.value.replace(/^#/, '') || '/').split('/').filter(Boolean)
    if (parts[0] === 'kp' && parts[1]) return { name: 'kp', id: parts[1] }
    if (parts[0] === 'module' && parts[1]) return { name: 'module', id: parts[1] }
    if (parts[0] === 'ai') return { name: 'ai' }
    return { name: 'home' }
  })
}

export function go(path) {
  window.location.hash = path
}

// ---------- 课程清单（单例缓存） ----------
const manifest = ref(null)
const manifestError = ref('')

export function useManifest() {
  async function load() {
    if (manifest.value || manifestError.value) return
    try {
      const res = await fetch('/api/manifest')
      if (!res.ok) throw new Error(`HTTP ${res.status}`)
      manifest.value = await res.json()
    } catch (e) {
      manifestError.value = `无法加载课程数据：${e.message}（请确认 rustway 服务已启动）`
    }
  }
  load()

  // 扁平化：按模块 → 章节 → 知识点的学习顺序
  const flatKps = computed(() => {
    if (!manifest.value) return []
    const list = []
    for (const m of manifest.value.modules) {
      for (const c of m.chapters) {
        for (const k of c.kps) list.push({ ...k, moduleName: m.title, moduleColor: m.color })
      }
    }
    return list
  })

  const kpIndex = computed(() => {
    const map = {}
    for (const k of flatKps.value) map[k.id] = k
    return map
  })

  return { manifest, manifestError, flatKps, kpIndex }
}

// ---------- 学习进度（localStorage 单例） ----------
const PROGRESS_KEY = 'rustway.progress.v1'

function loadProgress() {
  try {
    return JSON.parse(localStorage.getItem(PROGRESS_KEY)) || { read: {}, mastered: {}, quiz: {} }
  } catch {
    return { read: {}, mastered: {}, quiz: {} }
  }
}

const progress = ref(loadProgress())

function persist() {
  localStorage.setItem(PROGRESS_KEY, JSON.stringify(progress.value))
}

export function useProgress() {
  function markRead(id) {
    if (!progress.value.read[id]) {
      progress.value.read[id] = Date.now()
      persist()
    }
  }
  function setMastered(id, on) {
    if (on) progress.value.mastered[id] = Date.now()
    else delete progress.value.mastered[id]
    persist()
  }
  function setQuizBest(id, pct) {
    const cur = progress.value.quiz[id] || 0
    if (pct > cur) {
      progress.value.quiz[id] = pct
      persist()
    }
  }
  const masteredCount = computed(() => Object.keys(progress.value.mastered).length)
  return { progress, markRead, setMastered, setQuizBest, masteredCount }
}

// ---------- 明暗主题 ----------
const theme = ref(document.documentElement.dataset.theme || 'dark')

export function useTheme() {
  function toggle() {
    theme.value = theme.value === 'dark' ? 'light' : 'dark'
    document.documentElement.dataset.theme = theme.value
    localStorage.setItem('rustway.theme', theme.value)
  }
  return { theme, toggle }
}

// ---------- 数字滚动动效（count-up） ----------
const REDUCED =
  typeof window.matchMedia === 'function' &&
  window.matchMedia('(prefers-reduced-motion: reduce)').matches

export function useCountUp(getTarget, duration = 900) {
  const display = ref(0)
  let raf = 0
  watch(
    getTarget,
    (to, from) => {
      cancelAnimationFrame(raf)
      const b = Number(to) || 0
      if (REDUCED) {
        display.value = b
        return
      }
      const a = Number(from) || 0
      const start = performance.now()
      const step = t => {
        const p = Math.min((t - start) / duration, 1)
        display.value = Math.round(a + (b - a) * (1 - Math.pow(1 - p, 3)))
        if (p < 1) raf = requestAnimationFrame(step)
      }
      raf = requestAnimationFrame(step)
    },
    { immediate: true }
  )
  return display
}

// ---------- 延迟点火器（进度条生长等 CSS 过渡的触发） ----------
export function useDelayedFlag(ms = 60) {
  const on = ref(false)
  onMounted(() => setTimeout(() => (on.value = true), ms))
  return on
}
