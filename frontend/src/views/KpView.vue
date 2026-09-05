<script setup>
import { ref, computed, watch } from 'vue'
import { useManifest, useProgress, go } from '../composables/store'
import QuizWidget from '../components/QuizWidget.vue'

const props = defineProps({ id: { type: String, required: true } })
const { manifest, kpIndex } = useManifest()
const { progress, markRead, setMastered, setQuizBest } = useProgress()

const kp = ref(null)
const loading = ref(true)

async function fetchKp(id) {
  loading.value = true
  kp.value = null
  try {
    const res = await fetch('/api/kp/' + encodeURIComponent(id))
    if (!res.ok) throw new Error('HTTP ' + res.status)
    kp.value = await res.json()
    markRead(id)
  } catch (e) {
    kp.value = { id, title: '知识点不存在', summary: String(e), refs: [], quiz: [], prereqs: [], outline: [], tags: [] }
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

    <div v-else-if="kp" class="kp-layout">
      <div>
        <div class="card page-head stagger">
          <div class="meta-row">
            <span v-if="meta" class="chip"><span class="dot" :style="{ background: meta.moduleColor }" />{{ meta.moduleName }}</span>
            <span class="chip">{{ stars(kp.difficulty) }} {{ ['', '入门', '中阶', '进阶'][kp.difficulty] }}</span>
            <span class="chip">⏱ 约 {{ kp.minutes }} 分钟</span>
            <span v-for="t in kp.tags" :key="t" class="chip">#{{ t }}</span>
            <span v-if="progress.mastered[kp.id]" class="chip" style="color: var(--gold); border-color: var(--gold)">🏆 已掌握</span>
          </div>
          <h1>{{ kp.title }}</h1>
          <p class="summary">{{ kp.summary }}</p>

          <div v-if="kp.prereqs.length" style="margin-top: 12px">
            <div class="section-title" style="font-size: 12.5px">🔗 前置知识</div>
            <div class="prereq-list">
              <span v-for="p in kp.prereqs" :key="p" class="chip" @click="go('/kp/' + p)">{{ p }}</span>
            </div>
          </div>
        </div>

        <div v-if="kp.detailHtml" class="card md-body stagger" style="animation-delay: 60ms" v-html="kp.detailHtml" />
        <div v-else class="card outline-box stagger" style="animation-delay: 60ms">
          <div class="section-title">📖 知识点大纲</div>
          <ul>
            <li v-for="(o, i) in kp.outline" :key="i">{{ o }}</li>
          </ul>
          <p class="empty-tip" style="margin-top: 10px">
            ✍️ 详细讲解持续编写中——大纲、验证任务、测验与引用出处已就绪，不影响按任务实践学习。
          </p>
        </div>

        <div class="callout stagger" style="animation-delay: 100ms">
          <div class="c-title">✅ 验证任务（动手才算学会）</div>
          <p>{{ kp.task }}</p>
        </div>

        <div class="card info-card stagger" style="animation-delay: 140ms; margin-top: 16px">
          <div class="section-title">📝 自测题（先作答，再看解析）</div>
          <QuizWidget :key="kp.id" :questions="kp.quiz" :kp-id="kp.id" @update="onQuizUpdate" />
        </div>

        <div v-if="siblings" class="stagger" style="display: flex; gap: 10px; margin-top: 16px">
          <button v-if="siblings.prev" class="master-btn" @click="go('/kp/' + siblings.prev.id)">← {{ siblings.prev.title }}</button>
          <button v-if="siblings.next" class="master-btn" style="margin-left: auto" @click="go('/kp/' + siblings.next.id)">{{ siblings.next.title }} →</button>
        </div>
      </div>

      <aside>
        <div class="card side-card stagger" style="animation-delay: 80ms">
          <div class="section-title">📚 引用出处（可验证）</div>
          <ul class="ref-list">
            <li v-for="(r, i) in kp.refs" :key="i">
              <a :href="r.u" target="_blank" rel="noopener">{{ r.t }} ↗</a>
              <div style="color: var(--faint); font-size: 11px">{{ r.u }}</div>
            </li>
          </ul>
        </div>

        <div class="card side-card stagger" style="animation-delay: 120ms">
          <div class="section-title">🎯 掌握状态</div>
          <p class="empty-tip" style="margin-bottom: 10px">
            通过本页全部自测题即可自动点亮；也可在完成验证任务后手动标记。
          </p>
          <button class="master-btn" :class="{ on: progress.mastered[kp.id] }" @click="setMastered(kp.id, !progress.mastered[kp.id])">
            {{ progress.mastered[kp.id] ? '🏆 已掌握（点击取消）' : '标记为已掌握' }}
          </button>
        </div>

        <div v-if="siblings" class="card side-card stagger" style="animation-delay: 160ms">
          <div class="section-title">🏁 本章关卡测验</div>
          <p class="empty-tip">完成本章全部知识点后，回到模块页通过关卡测验检验学习成果。</p>
          <button class="master-btn" style="margin-top: 10px" @click="go('/module/' + meta.moduleId)">进入 {{ meta.moduleName }} →</button>
        </div>
      </aside>
    </div>
  </div>
</template>
