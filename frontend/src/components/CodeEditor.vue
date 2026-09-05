<script setup>
/**
 * CodeMirror 6 代码编辑器（编程题作答）：
 * Rust 语法高亮 / 行号 / 撤销历史 / Tab 缩进；明暗主题联动；v-model 双向绑定。
 */
import { ref, onMounted, onBeforeUnmount, watch } from 'vue'
import { EditorView, keymap, lineNumbers, highlightActiveLineGutter } from '@codemirror/view'
import { EditorState, Compartment } from '@codemirror/state'
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands'
import { rust } from '@codemirror/lang-rust'
import { oneDark } from '@codemirror/theme-one-dark'
import { useTheme } from '../composables/store'

const props = defineProps({
  modelValue: { type: String, default: '' },
  language: { type: String, default: 'rust' },
})
const emit = defineEmits(['update:modelValue'])

const host = ref(null)
const themeComp = new Compartment()
let view = null
let applying = false

function themeExt() {
  return (document.documentElement.dataset.theme || 'dark') === 'dark' ? oneDark : []
}

onMounted(() => {
  view = new EditorView({
    parent: host.value,
    state: EditorState.create({
      doc: props.modelValue,
      extensions: [
        lineNumbers(),
        highlightActiveLineGutter(),
        history(),
        keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
        rust(),
        themeComp.of(themeExt()),
        EditorView.lineWrapping,
        EditorView.updateListener.of(u => {
          if (!u.docChanged) return
          applying = true
          emit('update:modelValue', u.state.doc.toString())
          applying = false
        }),
      ],
    }),
  })
})

onBeforeUnmount(() => view?.destroy())

// 外部值变化（重做/重置）→ 同步进编辑器
watch(
  () => props.modelValue,
  v => {
    if (applying || !view) return
    const cur = view.state.doc.toString()
    if (v !== cur) view.dispatch({ changes: { from: 0, insert: v } })
  }
)

// 明暗主题切换 → 换主题扩展
const { theme } = useTheme()
watch(theme, () => {
  view?.dispatch({
    effects: themeComp.reconfigure(theme.value === 'dark' ? oneDark : []),
  })
})
</script>

<template>
  <div class="cm-host">
    <div ref="host" />
  </div>
</template>

<style>
.cm-host { border: 1px solid var(--border); border-radius: 10px; overflow: hidden; }
.cm-host .cm-editor { background: var(--code-bg) !important; font-size: 13px; }
.cm-host .cm-editor.cm-focused { outline: 1px solid var(--accent); }
.cm-host .cm-gutters { background: var(--panel-soft) !important; border-right: 1px solid var(--border-soft) !important; }
</style>
