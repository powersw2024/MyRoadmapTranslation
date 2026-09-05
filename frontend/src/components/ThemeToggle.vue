<script setup>
import { useTheme } from '../composables/store'

const { theme, toggle } = useTheme()
</script>

<template>
  <button class="theme-toggle" :title="theme === 'dark' ? '切换到亮色模式' : '切换到暗色模式'" @click="toggle">
    <Transition name="page" mode="out-in">
      <span :key="theme">{{ theme === 'dark' ? '🌙' : '☀️' }}</span>
    </Transition>
  </button>
</template>
