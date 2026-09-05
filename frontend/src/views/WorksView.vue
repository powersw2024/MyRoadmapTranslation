<script setup>
/**
 * 我的作品：浏览已入库的编程题/主观题提交（知识点 ↔ 文件 ↔ 评测/AI 结果映射）。
 */
import { ref, computed, onMounted } from 'vue'
import { useManifest, go } from '../composables/store'

const { manifest, kpIndex } = useManifest()
const items = ref([])
const loading = ref(true)
const err = ref('')
const filter = ref('all') // all | code | subjective
const detail = ref(null)

const kpTitle = id => kpIndex.value[id]?.title || id

const filtered = computed(() =>
  filter.value === 'all' ? items.value : items.value.filter(x => x.kind === filter.value)
)
const kindCount = kind => items.value.filter(x => x.kind === kind).length

async function load() {
  loading.value = true
  err.value = ''
  try {
    const res = await fetch('/api/submissions')
    if (!res.ok) throw new Error(`HTTP ${res.status}`)
    const d = await res.json()
    items.value = (d.submissions || []).map(s => ({
      ...s,
      kind: s.kind || (s.file_path.endsWith('.rs') ? 'code' : 'subjective'),
    }))
  } catch (e) {
    err.value = String(e.message || e)
  }
  loading.value = false
}
onMounted(load)

async function openDetail(id) {
  try {
    const res = await fetch('/api/submissions/' + id)
    detail.value = await res.json()
  } catch (e) {
    detail.value = { code: String(e.message || e) }
  }
}
function closeDetail() {
  detail.value = null
}
function fmtTs(ts) {
  const m = String(ts || '').match(/(\d{13})/)
  return m ? new Date(Number(m[1])).toLocaleString('zh-CN') : ts
}
</script>

<template>
  <div>
    <div class="breadcrumb stagger">
      <a href="#/">首页</a><span class="sep">/</span><span>我的作品</span>
    </div>

    <div class="card page-head pad-lg stagger">
      <div class="meta-row">
        <span class="chip">🗂️ 本地留档</span>
        <span class="chip">{{ items.length }} 条提交</span>
      </div>
      <h1 class="t-page">🗂️ 我的作品</h1>
      <p class="summary t-body c-muted">
        通过评测的编程题与主观题答案自动留档到本地数据库——这里是「知识点 ↔ 代码 ↔ 评测结果」的总账。
      </p>
    </div>

    <div class="card info-card pad-lg stagger" style="animation-delay: 60ms">
      <div v-if="loading" class="loading">加载中…</div>
      <div v-else-if="err" class="c-bad">{{ err }}</div>
      <template v-else>
        <div class="category-tabs" style="margin: 0 0 14px">
          <span class="chip cat-tab" :class="{ active: filter === 'all' }" @click="filter = 'all'">
            全部（{{ items.length }}）
          </span>
          <span class="chip cat-tab" :class="{ active: filter === 'code' }" @click="filter = 'code'">
            编程（{{ kindCount('code') }}）
          </span>
          <span class="chip cat-tab" :class="{ active: filter === 'subjective' }" @click="filter = 'subjective'">
            主观（{{ kindCount('subjective') }}）
          </span>
        </div>

        <div v-if="!filtered.length" class="empty-tip">
          还没有留档作品。完成任意编程题（运行测试通过）或主观题提交后，会自动出现在这里。
        </div>

        <ul v-else class="works-list">
          <li v-for="s in filtered" :key="s.id" class="work-item" @click="openDetail(s.id)">
            <span class="work-icon">{{ s.kind === 'code' ? '🦀' : '✍️' }}</span>
            <span class="work-title c-text">{{ kpTitle(s.kp_id) }}</span>
            <span class="chip" :class="s.passed ? 'c-ok' : 'c-bad'" style="font-size: 11px">
              {{ s.passed ? '通过' : '未通过' }}
            </span>
            <span v-if="s.ai_score !== null && s.ai_score !== undefined" class="chip c-gold" style="font-size: 11px">
              AI {{ s.ai_score }} 分
            </span>
            <span class="t-caption c-faint work-ts">{{ fmtTs(s.created_at) }}</span>
          </li>
        </ul>
      </template>
    </div>

    <!-- 详情弹层 -->
    <Teleport to="body">
      <div v-if="detail" class="detail-mask" @click.self="closeDetail">
        <div class="card detail-panel">
          <div class="detail-head">
            <b>{{ kpTitle(detail.kpId) || '作品详情' }}</b>
            <button class="btn btn-sm" @click="closeDetail">关闭 ✕</button>
          </div>
          <pre class="detail-code">{{ detail.code }}</pre>
          <div v-if="detail.aiComments" class="quiz-why" style="margin-top: 12px">
            <b>AI 评语：</b>{{ detail.aiComments }}
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.works-list { list-style: none; }
.work-item {
  display: flex; align-items: center; gap: 10px;
  padding: 10px 12px; border-radius: 10px; cursor: pointer;
  border-bottom: 1px solid var(--border-soft);
  transition: background 0.15s, transform 0.15s;
  font-size: 13.5px;
}
.work-item:hover { background: var(--panel-hover); transform: translateX(3px); }
.work-icon { font-size: 15px; }
.work-title { flex: 1; font-weight: 600; }
.work-ts { white-space: nowrap; }
.detail-mask {
  position: fixed; inset: 0; z-index: 120;
  background: rgba(0, 0, 0, 0.5);
  display: grid; place-items: center;
  backdrop-filter: blur(4px);
}
.detail-panel {
  width: min(860px, 92vw); max-height: 82vh; overflow: auto;
  padding: 20px 22px;
  animation: fadeSlide 0.3s var(--ease);
}
.detail-head { display: flex; align-items: center; justify-content: space-between; margin-bottom: 14px; }
.detail-code {
  background: var(--code-bg); border: 1px solid var(--border);
  border-radius: 10px; padding: 14px;
  font-family: var(--font-mono); font-size: 12.5px; line-height: 1.7;
  white-space: pre-wrap; word-break: break-word;
}
</style>
