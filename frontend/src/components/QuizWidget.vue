<script setup>
import { ref, computed } from 'vue'

const props = defineProps({
  questions: { type: Array, required: true }, // [{q, opts, answer, why, kp?}]
  kpId: { type: String, default: '' },        // 提供时自动记录测验成绩
  title: { type: String, default: '自测题' },
})
const emit = defineEmits(['passed', 'update'])

const picks = ref(props.questions.map(() => null)) // 选中的选项下标

const allAnswered = computed(() => picks.value.every(p => p !== null))
const correctCount = computed(
  () => picks.value.filter((p, i) => p === props.questions[i].answer).length
)
const allCorrect = computed(() => correctCount.value === props.questions.length)

function pick(qi, oi) {
  if (picks.value[qi] !== null) return
  picks.value[qi] = oi
  emit('update', { correctCount: correctCount.value, allCorrect: allCorrect.value, allAnswered: allAnswered.value, total: props.questions.length })
  if (allCorrect.value) emit('passed')
}

defineExpose({ correctCount, allAnswered, allCorrect })
</script>

<template>
  <div class="quiz">
    <div v-for="(q, qi) in questions" :key="qi" class="card quiz-card">
      <div class="quiz-q">{{ qi + 1 }}. {{ q.q }}</div>
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
          <span style="opacity: 0.6; margin-right: 8px">{{ 'ABCD'[oi] }}</span>{{ opt }}
          <span v-if="picks[qi] !== null && oi === q.answer" class="mark" style="color: var(--ok)">✓</span>
          <span v-else-if="picks[qi] === oi" class="mark" style="color: var(--bad)">✗</span>
        </button>
      </div>
      <div v-if="picks[qi] !== null" class="quiz-why">
        <b>解析：</b>{{ q.why }}
      </div>
    </div>

    <div v-if="questions.length && allAnswered" class="quiz-why" style="border-left-color: var(--accent)">
      <b v-if="allCorrect" style="color: var(--ok)">🎉 全部正确 —— 该组知识点已标记为掌握！</b>
      <b v-else>得分 {{ correctCount }}/{{ questions.length }} —— 答错的题目建议隔天重做（间隔重复）。</b>
    </div>
  </div>
</template>
