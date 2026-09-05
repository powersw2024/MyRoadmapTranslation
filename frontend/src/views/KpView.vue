<script setup>
import { ref, computed, watch, nextTick, onMounted, onBeforeUnmount } from 'vue'
import { useManifest, useProgress, setLastKp, go } from '../composables/store'
import QuizWidget from '../components/QuizWidget.vue'
import hljs from 'highlight.js/lib/core'
import rust from 'highlight.js/lib/languages/rust'
import python from 'highlight.js/lib/languages/python'
import javascript from 'highlight.js/lib/languages/javascript'
import bash from 'highlight.js/lib/languages/bash'
import json from 'highlight.js/lib/languages/json'
import sql from 'highlight.js/lib/languages/sql'

hljs.registerLanguage('rust', rust)
hljs.registerLanguage('python', python)
hljs.registerLanguage('javascript', javascript)
hljs.registerLanguage('bash', bash)
hljs.registerLanguage('json', json)
hljs.registerLanguage('sql', sql)

const props = defineProps({ id: { type: String, required: true } })
const { manifest, kpIndex } = useManifest()
const { progress, markRead, setMastered, setQuizBest } = useProgress()

const kp = ref(null)
const kpError = ref('')
const loading = ref(true)
const bodyEl = ref(null)
const toc = ref([])
const activeH = ref('')

async function fetchKp(id) {
  loading.value = true
  kp.value = null
  kpError.value = ''
  try {
    const res = await fetch('/api/kp/' + encodeURIComponent(id))
    if (!res.ok) throw new Error(res.status === 404 ? '知识点不存在或已被移动' : `服务异常（HTTP ${res.status}）`)
    kp.value = await res.json()
    markRead(id)
    if (kp.value.title) setLastKp(id, kp.value.title)
    await nextTick()
    enhanceBody()
  } catch (e) {
    kpError.value = String(e.message || e)
  }
  loading.value = false
}
watch(() => props.id, fetchKp, { immediate: true })

/** 正文增强：语法高亮 / 复制按钮 / 提取 h2-h3 生成目录 */
function enhanceBody() {
  const el = bodyEl.value
  if (!el) return
  el.querySelectorAll('pre code').forEach(block => {
    if (!block.dataset.hl) {
      hljs.highlightElement(block)
      block.dataset.hl = '1'
    }
    const pre = block.parentElement
    if (pre && !pre.querySelector('.copy-btn')) {
      const btn = document.createElement('button')
      btn.className = 'copy-btn'
      btn.textContent = '复制'
      btn.addEventListener('click', async () => {
        try {
          await navigator.clipboard.writeText(block.textContent)
          btn.textContent = '已复制 ✓'
          btn.classList.add('copied')
          setTimeout(() => {
            btn.textContent = '复制'
            btn.classList.remove('copied')
          }, 1500)
        } catch {
          btn.textContent = '复制失败'
        }
      })
      pre.appendChild(btn)
    }
  })
  // 目录：h2/h3 提取
  const items = []
  el.querySelectorAll('h2, h3').forEach((h, i) => {
    const id = 'sec-' + i
    h.id = id
    items.push({ id, text: h.textContent, lvl: h.tagName === 'H2' ? 2 : 3 })
  })
  toc.value = items
}

function scrollToSec(id) {
  document.getElementById(id)?.scrollIntoView({ behavior: 'smooth', block: 'start' })
}

// ←/→ 切换同章前后知识点（输入框聚焦时不触发）
function onKey(e) {
  const tag = (e.target.tagName || '').toLowerCase()
  if (['input', 'textarea', 'select'].includes(tag) || e.target.isContentEditable) return
  if (!siblings.value) return
  if (e.key === 'ArrowLeft' && siblings.value.prev) go('/kp/' + siblings.value.prev.id)
  if (e.key === 'ArrowRight' && siblings.value.next) go('/kp/' + siblings.value.next.id)
}
onMounted(() => window.addEventListener('keydown', onKey))
onBeforeUnmount(() => window.removeEventListener('keydown', onKey))

const meta = computed(() => kpIndex.value[props.id] || kp.value)

const siblings = computed(() => {
  if (!meta.value?.chapterId) return null
  for (const m of manifest.value?.modules || []) {
    for (const c of m.chapters) {
      if (c.id === meta.value.chapterId) {
        const idx = c.kps.findIndex(k => k.id === props.id)
        return {
          prev: idx > 0 ? c.kps[idx - 1] : null,
          next: idx < c.kps.length - 1 ? c.kps[idx + 1] : null,
          chapter: c,
        }
      }
    }
  }
  return null
})

function onQuizUpdate(payload) {
  if (!kp.value || !payload.total) return
  setQuizBest(kp.value.id, Math.round((payload.correctCount / payload.total) * 100))
  if (payload.allCorrect) setMastered(kp.value.id, true)
}

function stars(d) {
  return '★'.repeat(d) + '☆'.repeat(3 - d)
}
</script>

<template>
  <div>
    <div class="breadcrumb stagger">
      <a href="#/">首页</a>
      <template v-if="meta">
        <span class="sep">/</span>
        <a :href="'#/module/' + meta.moduleId" @click.prevent="go('/module/' + meta.moduleId)">{{ meta.moduleName }}</a>
        <span class="sep">/</span>
        <span>{{ meta.title }}</span>
      </template>
    </div>

    <div v-if="loading" class="loading">加载知识点…</div>

    <div v-else-if="kpError" class="card notfound stagger">
      <div class="nf-icon">🧭</div>
      <h2 class="t-item">没有找到这个知识点</h2>
      <p class="t-desc c-muted">{{ kpError }}<br />可能链接有误，或它在「待编排」区还未编入章节。</p>
      <button class="btn" @click="go('/')">← 回到知识图谱</button>
    </div>

    <div v-else-if="kp" class="kp-layout">
      <div>
        <div class="card page-head pad-lg stagger">
          <div class="meta-row">
            <span v-if="meta" class="chip"><span class="dot" style="background: var(--accent)" />{{ meta.moduleName }}</span>
            <span class="chip">{{ stars(kp.difficulty) }} {{ ['', '入门', '中阶', '进阶'][kp.difficulty] }}</span>
            <span class="chip">⏱ 约 {{ kp.minutes }} 分钟</span>
            <span v-for="t in kp.tags" :key="t" class="chip">#{{ t }}</span>
            <span v-if="progress.mastered[kp.id]" class="chip c-gold" style="border-color: var(--gold)">🏆 已掌握</span>
          </div>
          <h1 class="t-page">{{ kp.title }}</h1>
          <p class="summary t-body c-muted">{{ kp.summary }}</p>

          <div v-if="kp.prereqs.length" class="gap-t">
            <div class="section-title">🔗 前置知识</div>
            <div class="prereq-list">
              <span v-for="p in kp.prereqs" :key="p" class="chip" @click="go('/kp/' + p)">{{ p }}</span>
            </div>
          </div>
          <div v-if="kp.related?.length" class="gap-t">
            <div class="section-title">🧠 相似关联（算法推荐）</div>
            <div class="prereq-list">
              <span v-for="r in kp.related" :key="r.id" class="chip" @click="go('/kp/' + r.id)">
                {{ r.title }} <span class="t-caption t-num c-muted">{{ r.score }}%</span>
              </span>
            </div>
          </div>
        </div>

        <div ref="bodyEl" v-if="kp.detailHtml" class="card md-body stagger" style="animation-delay: 60ms" v-html="kp.detailHtml" />
        <div v-else class="card outline-box pad stagger" style="animation-delay: 60ms">
          <div class="section-title">📖 知识点大纲</div>
          <ul>
            <li v-for="(o, i) in kp.outline" :key="i" class="c-muted">{{ o }}</li>
          </ul>
          <p class="empty-tip gap-t">
            ✍️ 详细讲解持续编写中——大纲、验证任务、测验与引用出处已就绪，不影响按任务实践学习。
          </p>
        </div>

        <div class="callout stagger" style="animation-delay: 100ms">
          <div class="c-title">✅ 验证任务（动手才算学会）</div>
          <p class="t-desc">{{ kp.task }}</p>
        </div>

        <div class="card info-card pad stagger gap-t" style="animation-delay: 140ms">
          <div class="section-title">📝 自测题（先作答，再看解析）</div>
          <QuizWidget :key="kp.id" :questions="kp.quiz" :kp-id="kp.id" @update="onQuizUpdate" />
        </div>

        <div v-if="siblings" class="stagger gap-t" style="display: flex; gap: 10px">
          <button v-if="siblings.prev" class="btn" @click="go('/kp/' + siblings.prev.id)">← {{ siblings.prev.title }}</button>
          <button v-if="siblings.next" class="btn" style="margin-left: auto" @click="go('/kp/' + siblings.next.id)">{{ siblings.next.title }} →</button>
        </div>
      </div>

      <aside>
        <div v-if="toc.length >= 3" class="card side-card pad stagger" style="animation-delay: 40ms">
          <div class="section-title">📑 本页目录</div>
          <ul class="toc-list">
            <li
              v-for="t in toc"
              :key="t.id"
              :class="t.lvl === 2 ? '' : 'lvl2'"
            >
              <a :href="'#' + t.id" @click.prevent="scrollToSec(t.id)">{{ t.text }}</a>
            </li>
          </ul>
          <p class="form-hint" style="margin-top: 8px">提示：← → 键切换本课前后知识点</p>
        </div>

        <div class="card side-card pad stagger" style="animation-delay: 80ms">
          <div class="section-title">📚 引用出处（可验证）</div>
          <ul class="ref-list">
            <li v-for="(r, i) in kp.refs" :key="i">
              <a :href="r.u" target="_blank" rel="noopener">{{ r.t }} ↗</a>
              <div class="t-caption c-faint" style="margin-top: 2px">{{ r.u }}</div>
            </li>
          </ul>
        </div>

        <div class="card side-card pad stagger" style="animation-delay: 120ms">
          <div class="section-title">🎯 掌握状态</div>
          <p class="empty-tip stack-6">
            通过本页全部自测题即可自动点亮；也可在完成验证任务后手动标记。
          </p>
          <button class="btn gap-t" :class="{ 'btn-solid': progress.mastered[kp.id] }" @click="setMastered(kp.id, !progress.mastered[kp.id])">
            {{ progress.mastered[kp.id] ? '🏆 已掌握（点击取消）' : '标记为已掌握' }}
          </button>
        </div>

        <div v-if="siblings" class="card side-card pad stagger" style="animation-delay: 160ms">
          <div class="section-title">🏁 本章关卡测验</div>
          <p class="empty-tip">完成本章全部知识点后，回到模块页通过关卡测验检验学习成果。</p>
          <button class="btn gap-t" @click="go('/module/' + meta.moduleId)">进入 {{ meta.moduleName }} →</button>
        </div>
      </aside>
    </div>
  </div>
</template>
