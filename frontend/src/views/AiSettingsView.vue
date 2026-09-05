<script setup>
/**
 * AI 状态页：展示服务端环境变量配置的评价后端（只读），
 * 「测试连接」用当前生效配置发起一次最小补全。
 * 安全契约：后端地址只由服务端环境变量决定，页面不接受/不传任何 URL。
 */
import { ref, onMounted } from 'vue'

const backend = ref('')
const baseUrl = ref('')
const model = ref('')
const apiKeySet = ref(false)
const source = ref('none')

const loading = ref(true)
const loadMsg = ref('')
const testing = ref(false)
const testResult = ref(null) // { ok, latencyMs? , error? }

async function load() {
  loading.value = true
  try {
    const res = await fetch('/api/ai/config')
    const d = await res.json()
    backend.value = d.backend || ''
    baseUrl.value = d.baseUrl || ''
    model.value = d.model || ''
    apiKeySet.value = !!d.apiKeySet
    source.value = d.source || 'none'
  } catch (e) {
    loadMsg.value = String(e.message || e)
  }
  loading.value = false
}
onMounted(load)

async function testConnection() {
  testing.value = true
  testResult.value = null
  try {
    const res = await fetch('/api/ai/probe', { method: 'POST' })
    testResult.value = await res.json()
  } catch (e) {
    testResult.value = { ok: false, error: String(e.message || e) }
  }
  testing.value = false
}
</script>

<template>
  <div>
    <div class="breadcrumb stagger">
      <a href="#/">首页</a><span class="sep">/</span><span>AI 评价</span>
    </div>

    <div class="card page-head pad-lg stagger">
      <div class="meta-row">
        <span class="chip">{{ source === 'environment' ? '来源：环境变量' : '未配置' }}</span>
        <span v-if="apiKeySet" class="chip c-ok">密钥已配置</span>
      </div>
      <h1 class="t-page">🤖 AI 代码评价</h1>
      <p class="summary t-body c-muted">
        为编程题提供 AI 代码评价。后端通过服务端环境变量配置（RUSTWAY_AI_BACKEND /
        RUSTWAY_AI_BASE_URL / RUSTWAY_AI_MODEL / RUSTWAY_AI_API_KEY），
        页面只读展示，不接受任何地址输入。不配置也不影响编程题的测试判分。
      </p>
    </div>

    <div class="card info-card pad-lg stagger" style="animation-delay: 60ms; max-width: 720px">
      <div v-if="loading" class="loading">加载配置…</div>
      <template v-else>
        <div class="cfg-row"><span class="cfg-label">后端</span><b>{{ backend || '—' }}</b></div>
        <div class="cfg-row"><span class="cfg-label">接口地址</span><code>{{ baseUrl || '—' }}</code></div>
        <div class="cfg-row"><span class="cfg-label">模型</span><b>{{ model || '—' }}</b></div>
        <div class="cfg-row"><span class="cfg-label">密钥</span><b>{{ apiKeySet ? '已配置' : '未配置' }}</b></div>
        <p v-if="loadMsg" class="t-note c-bad">{{ loadMsg }}</p>

        <div class="form-actions">
          <button class="btn btn-solid" :disabled="testing || source === 'none'" @click="testConnection">
            {{ testing ? '⏳ 测试中…' : '🔎 测试连接' }}
          </button>
        </div>

        <div v-if="testResult" class="test-result" :class="testResult.ok ? 'ok' : 'bad'">
          <template v-if="testResult.ok">
            <b>✓ 连接成功</b> —— 模型 {{ testResult.model }}，延迟 {{ testResult.latencyMs }}ms
          </template>
          <template v-else>
            <b>✗ 连接失败</b> —— {{ testResult.error }}
            <p class="form-hint" style="margin-top: 6px">
              排查：本地 Ollama 是否已启动（ollama serve）？模型是否已拉取（ollama pull &lt;模型名&gt;）？
              环境变量是否正确设置？
            </p>
          </template>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.cfg-row { display: flex; align-items: baseline; gap: 14px; padding: 9px 0; border-bottom: 1px dashed var(--border); }
.cfg-row:last-of-type { border-bottom: none; }
.cfg-label { width: 72px; flex: none; font-size: 12.5px; color: var(--faint); }
.cfg-row code { font-size: 13px; color: var(--text); word-break: break-all; }
.form-hint { font-size: 12px; color: var(--faint); line-height: 1.6; margin: 0; }
.form-actions { display: flex; align-items: center; gap: 12px; margin-top: 16px; flex-wrap: wrap; }
.test-result {
  margin-top: 18px; padding: 14px 18px; border-radius: 12px;
  font-size: 13.5px; line-height: 1.7;
  animation: fadeSlide 0.4s var(--ease);
}
.test-result.ok { border: 1px solid color-mix(in srgb, var(--ok) 50%, transparent); background: color-mix(in srgb, var(--ok) 8%, var(--panel)); }
.test-result.bad { border: 1px solid color-mix(in srgb, var(--bad) 45%, transparent); background: color-mix(in srgb, var(--bad) 7%, var(--panel)); }
</style>
