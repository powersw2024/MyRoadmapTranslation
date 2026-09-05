<script setup>
import { ref, computed, watch } from 'vue'
import { useManifest, useProgress, go } from '../composables/store'
import QuizWidget from '../components/QuizWidget.vue'

const props = defineProps({ id: { type: String, required: true } })
const { manifest, kpIndex } = useManifest()
const { progress, markRead, setMastered, setQuizBest } = useProgress()

const kp = ref(null)
const kpError = ref('')
const loading = ref(true)

async function fetchKp(id) {
  loading.value = true
  kp.value = null
  kpError.value = ''
  try {
    const res = await fetch('/api/kp/' + encodeURIComponent(id))
    if (!res.ok) throw new Error(res.status === 404 ? '知识点不存在或已被移动' : `服务异常（HTTP ${res.status}）`)
    kp.value = await res.json()
    markRead(id)
  } catch (e) {
    kpError.value = String(e.message || e)
  }
  loading.value = false
}
watch(() => props.id, fetchKp, { immediate: true })

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
            <span v-if="meta" class="chip"><span class="dot" :style="{ background: meta.moduleColor }" />{{ meta.moduleName }}</span>
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
        </div>

        <div v-if="kp.detailHtml" class="card md-body stagger" style="animation-delay: 60ms" v-html="kp.detailHtml" />
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
