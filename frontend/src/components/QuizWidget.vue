<script setup>
import { ref, computed } from 'vue'

const props = defineProps({
  questions: { type: Array, required: true }, // [{q, opts, answer, why, kp?}]
  kpId: { type: String, default: '' },        // 提供时自动记录测验成绩
  title: { type: String, default: '自测题' },
})
const emit = defineEmits(['passed', 'update'])

const picks = ref(props.questions.map(() => null)) // 选中的选项下标

const allAnswered = computed(() => props.questions.length > 0 && picks.value.every(p => p !== null))
const correctCount = computed(
  () => picks.value.filter((p, i) => p === props.questions[i].answer).length
)
const allCorrect = computed(
  () => props.questions.length > 0 && correctCount.value === props.questions.length
)

function pick(qi, oi) {
  if (picks.value[qi] !== null) return
  picks.value[qi] = oi
  emit('update', { correctCount: correctCount.value, allCorrect: allCorrect.value, allAnswered: allAnswered.value, total: props.questions.length })
  if (allCorrect.value) emit('passed')
}

function reset() {
  picks.value = props.questions.map(() => null)
  emit('update', { correctCount: 0, allCorrect: false, allAnswered: false, total: props.questions.length })
}

defineExpose({ correctCount, allAnswered, allCorrect })
</script>

<template>
  <div class="quiz">
    <div v-for="(q, qi) in questions" :key="qi" class="card quiz-card">
      <div class="quiz-q t-desc c-text">{{ qi + 1 }}. {{ q.q }}</div>
      <div class="quiz-opts">
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
          <span class="c-faint quiz-key">{{ 'ABCD'[oi] }}</span>{{ opt }}
          <span v-if="picks[qi] !== null && oi === q.answer" class="mark c-ok">✓</span>
          <span v-else-if="picks[qi] === oi" class="mark c-bad">✗</span>
        </button>
      </div>
      <div v-if="picks[qi] !== null" class="quiz-why">
        <b>解析：</b>{{ q.why }}
      </div>
    </div>

    <div v-if="allAnswered" class="quiz-footer">
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
</style>
