<script setup>
/**
 * 测验组件：支持三种题型
 * - choice 选择题（kind 缺省）：点选即时判分
 * - blank  填空题：输入答案，服务端规则 = 忽略大小写与首尾空格后比对 accept 列表
 * - code   编程题：代码编辑区 + 「运行测试」→ POST /api/judge（rustc 沙箱编译运行），
 *   通过后自动落盘留档；若服务端配置了 AI 后端（Ollama/OpenAI 兼容）可请求代码评价
 */
import { ref, computed, watch } from 'vue'

const props = defineProps({
  questions: { type: Array, required: true },
  kpId: { type: String, default: '' },
  title: { type: String, default: '自测题' },
})
const emit = defineEmits(['passed', 'update'])

const picks = ref([])      // choice：选中下标
const blanks = ref([])     // blank：{ value, done, ok }
const codes = ref([])      // code：{ code, done, passed, output, running, submissionId, review, reviewing, reviewErr }
const aiEnabled = ref(false)
const aiChecked = ref(false)

function initState() {
  picks.value = props.questions.map(() => null)
  blanks.value = props.questions.map(() => ({ value: '', done: false, ok: false }))
  codes.value = props.questions.map(q => ({
    code: q.starter || '',
    done: false, passed: false, output: '', running: false,
    submissionId: null, review: null, reviewing: false, reviewErr: '',
  }))
}
watch(() => props.questions, initState, { immediate: true })

// AI 后端是否可用（服务端 /api/health 上报）
watch(() => props.kpId, checkAi, { immediate: true })
async function checkAi() {
  try {
    const res = await fetch('/api/health')
    const h = await res.json()
    aiEnabled.value = !!(h.ai && h.ai.enabled)
  } catch {
    aiEnabled.value = false
  }
  aiChecked.value = true
}

const norm = s => (s || '').trim().toLowerCase()

const graded = computed(() =>
  props.questions.map((q, i) => {
    if (q.kind === 'blank') return blanks.value[i].done
    if (q.kind === 'code') return codes.value[i].done
    return picks.value[i] !== null
  })
)
const allAnswered = computed(
  () => props.questions.length > 0 && graded.value.every(Boolean)
)
const isCorrect = computed(() =>
  props.questions.map((q, i) => {
    if (q.kind === 'blank') return blanks.value[i].ok
    if (q.kind === 'code') return codes.value[i].passed
    return picks.value[i] === q.answer
  })
)
const correctCount = computed(() => isCorrect.value.filter(Boolean).length)
const allCorrect = computed(
  () => props.questions.length > 0 && correctCount.value === props.questions.length
)

function notify() {
  emit('update', {
    correctCount: correctCount.value,
    allCorrect: allCorrect.value,
    allAnswered: allAnswered.value,
    total: props.questions.length,
  })
  if (allCorrect.value) emit('passed')
}

function pick(qi, oi) {
  if (picks.value[qi] !== null) return
  picks.value[qi] = oi
  notify()
  if (allCorrect.value) emit('passed')
}

function submitBlank(qi) {
  const q = props.questions[qi]
  const b = blanks.value[qi]
  if (b.done) return
  b.ok = q.accept.some(a => norm(a) === norm(b.value))
  b.done = true
  notify()
  if (allCorrect.value) emit('passed')
}

async function runCode(qi) {
  const q = props.questions[qi]
  const c = codes.value[qi]
  if (c.running) return
  c.running = true
  c.review = null
  try {
    const res = await fetch('/api/judge', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ kp_id: props.kpId || q.kp, code: c.code }),
    })
    const data = await res.json()
    c.done = true
    c.passed = !!data.passed
    c.output = data.output || ''
    c.submissionId = data.submissionId || null
    if (!c.passed && data.message) c.output = data.message
  } catch (e) {
    c.done = true
    c.passed = false
    c.output = String(e.message || e)
  }
  c.running = false
  notify()
  if (allCorrect.value) emit('passed')
  if (c.passed && aiEnabled.value && c.submissionId) requestReview(qi)
}

async function requestReview(qi) {
  const c = codes.value[qi]
  if (!c.submissionId || c.reviewing) return
  c.reviewing = true
  c.reviewErr = ''
  try {
    const res = await fetch('/api/review', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ submission_id: c.submissionId }),
    })
    if (!res.ok) {
      const d = await res.json().catch(() => ({}))
      throw new Error(d.message || `HTTP ${res.status}`)
    }
    c.review = await res.json()
  } catch (e) {
    c.reviewErr = String(e.message || e)
  }
  c.reviewing = false
}

function reset() {
  initState()
  notify()
}

function stateLabel(i) {
  if (!graded.value[i]) return ''
  return isCorrect.value[i] ? '✓ 通过' : '✗ 未通过'
}

defineExpose({ correctCount, allAnswered, allCorrect })
</script>

<template>
  <div class="quiz">
    <div v-for="(q, qi) in questions" :key="qi" class="card quiz-card">
      <!-- 题干 -->
      <div class="quiz-q t-desc c-text">
        {{ qi + 1 }}.
        <span class="chip quiz-kind">{{ { choice: '选择', blank: '填空', code: '编程' }[q.kind || 'choice'] }}</span>
        {{ q.kind === 'code' ? q.prompt : q.q }}
      </div>

      <!-- 选择题 -->
      <div v-if="(q.kind || 'choice') === 'choice'" class="quiz-opts">
        <button
          v-for="(opt, oi) in q.opts"
          :key="oi"
          class="quiz-opt"
          :class="{
            correct: picks[qi] !== null && oi === q.answer,
            wrong: picks[qi] === oi && oi !== q.answer,
          }"
          :disabled="picks[qi] !== null"
          @click="pick(qi, oi)"
        >
          <span class="quiz-key c-faint">{{ 'ABCD'[oi] }}</span>{{ opt }}
          <span v-if="picks[qi] !== null && oi === q.answer" class="mark c-ok">✓</span>
          <span v-else-if="picks[qi] === oi" class="mark c-bad">✗</span>
        </button>
      </div>

      <!-- 填空题 -->
      <div v-else-if="q.kind === 'blank'" class="quiz-blank">
        <input
          v-model="blanks[qi].value"
          class="blank-input"
          :disabled="blanks[qi].done"
          placeholder="填入空格处的答案"
          @keydown.enter="submitBlank(qi)"
        />
        <button class="btn btn-sm" :disabled="blanks[qi].done || !blanks[qi].value.trim()" @click="submitBlank(qi)">提交</button>
        <span v-if="blanks[qi].done" class="blank-verdict" :class="blanks[qi].ok ? 'c-ok' : 'c-bad'">
          {{ blanks[qi].ok ? '✓ 正确' : '✗ 不对，再想想' }}
        </span>
      </div>

      <!-- 编程题 -->
      <div v-else-if="q.kind === 'code'" class="quiz-code">
        <textarea
          v-model="codes[qi].code"
          class="code-editor"
          spellcheck="false"
          rows="9"
          placeholder="在此编写代码…"
        />
        <div class="code-actions">
          <button class="btn btn-sm" :disabled="codes[qi].running" @click="runCode(qi)">
            {{ codes[qi].running ? '⏳ 编译运行中…' : '▶ 运行测试' }}
          </button>
          <span v-if="codes[qi].done" class="code-verdict" :class="codes[qi].passed ? 'c-ok' : 'c-bad'">
            {{ stateLabel(qi) }}
          </span>
          <span v-if="codes[qi].passed" class="c-faint t-caption">已留档到本地数据库</span>
          <button
            v-if="aiEnabled && codes[qi].done && codes[qi].submissionId"
            class="btn btn-sm quiz-ai"
            :disabled="codes[qi].reviewing"
            @click="requestReview(qi)"
          >
            {{ codes[qi].reviewing ? '🤖 评价中…' : '🤖 AI 评价' }}
          </button>
        </div>
        <pre v-if="codes[qi].output" class="code-output" :class="{ ok: codes[qi].passed }">{{ codes[qi].output }}</pre>
        <div v-if="codes[qi].review || codes[qi].reviewErr" class="quiz-why">
          <template v-if="codes[qi].review">
            <b>🤖 AI 评价（{{ codes[qi].review.score }} 分）：</b>{{ codes[qi].review.summary }}
            <ul v-if="codes[qi].review.suggestions?.length" class="review-list">
              <li v-for="(s, si) in codes[qi].review.suggestions" :key="si">{{ s }}</li>
            </ul>
          </template>
          <template v-else><b>AI 评价失败：</b>{{ codes[qi].reviewErr }}</template>
        </div>
      </div>

      <!-- 解析 -->
      <div v-if="graded[qi]" class="quiz-why">
        <b>解析：</b>{{ q.why }}
      </div>
    </div>

    <div v-if="questions.length && allAnswered" class="quiz-footer">
      <span class="t-note c-muted">
        得分 <span class="score t-num" :class="allCorrect ? 'full' : 'part'">{{ correctCount }}/{{ questions.length }}</span>
      </span>
      <span v-if="allCorrect" class="t-note c-ok">🎉 全部正确 —— 已标记为掌握！</span>
      <span v-else class="t-note c-muted">答错的题目建议隔天重做（间隔重复）。</span>
      <button class="btn btn-sm quiz-retake" @click="reset">↻ 重做</button>
    </div>
  </div>
</template>

<style scoped>
.quiz-key { margin-right: 8px; }
.quiz-kind { margin: 0 6px; font-size: 11px; padding: 1px 8px; }
.blank-input {
  flex: 1; min-width: 180px;
  height: 36px; padding: 0 12px;
  border-radius: 10px; border: 1px solid var(--border);
  background: var(--panel-soft); color: var(--text);
  font-family: var(--font-mono); font-size: 13px; outline: none;
}
.blank-input:focus { border-color: var(--accent); }
.quiz-blank { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
.blank-verdict { font-size: 13px; font-weight: 700; }
.code-editor {
  width: 100%; resize: vertical;
  font-family: var(--font-mono); font-size: 13px; line-height: 1.7;
  background: var(--code-bg); color: var(--text);
  border: 1px solid var(--border); border-radius: 10px;
  padding: 12px 14px; outline: none;
}
.code-editor:focus { border-color: var(--accent); }
.code-actions { display: flex; align-items: center; gap: 10px; margin-top: 8px; flex-wrap: wrap; }
.code-verdict { font-weight: 700; font-size: 13px; }
.code-output {
  margin-top: 10px; padding: 10px 14px; border-radius: 10px;
  background: var(--code-bg); border: 1px solid var(--border);
  font-family: var(--font-mono); font-size: 12px; line-height: 1.6;
  white-space: pre-wrap; word-break: break-all;
  max-height: 260px; overflow: auto;
}
.code-output.ok { border-color: color-mix(in srgb, var(--ok) 50%, transparent); }
.review-list { margin: 6px 0 0; padding-left: 18px; }
.quiz-ai { margin-left: auto; }
</style>
