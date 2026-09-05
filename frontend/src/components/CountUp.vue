<script setup>
/**
 * 数字滚动组件：<CountUp :value="n" />
 * 数值变化时以 easeOutCubic 从旧值滚动到新值；prefers-reduced-motion 时直接显示。
 */
import { useCountUp } from '../composables/store'

const props = defineProps({ value: { type: Number, default: 0 } })
const display = useCountUp(() => props.value)
</script>

<template>{{ display }}</template>
