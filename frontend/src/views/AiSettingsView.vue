<script setup>
/**
 * AI 设置页：配置评价后端（本地 Ollama / OpenAI 兼容 API），
 * 保存到服务端 SQLite；「测试连接」用当前生效配置发起一次最小补全。
 * 密钥只提交、不回传（页面只显示"已配置"状态）。
 */
import { ref, onMounted } from 'vue'

const backend = ref('ollama')
const baseUrl = ref('')
const model = ref('')
const apiKey = ref('')
const apiKeySet = ref(false)
const source = ref('none')

const loading = ref(true)
const saving = ref(false)
const saveMsg = ref('')
const testing = ref(false)
const testResult = ref(null) // { ok, latencyMs? , error? }

async function load() {
  loading.value = true
  try {
    const res = await fetch('/api/ai/config')
    const d = await res.json()
    backend.value = d.backend || 'ollama'
    baseUrl.value = d.baseUrl || ''
    model.value = d.model || ''
    apiKeySet.value = !!d.apiKeySet
    source.value = d.source || 'none'
  } catch (e) {
    saveMsg.value = String(e.message || e)
  }
  loading.value = false
}
onMounted(load)

async function save() {
  saving.value = true
  saveMsg.value = ''
  try {
    const res = await fetch('/api/ai/config', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        backend: backend.value,
        base_url: baseUrl.value,
        model: model.value,
        api_key: apiKey.value || null,
        clear_api_key: apiKey.value === '__CLEAR__',
      }),
    })
    const d = await res.json()
    if (!res.ok) throw new Error(d.message || `HTTP ${res.status}`)
    saveMsg.value = '✓ 已保存'
    apiKey.value = ''
    await load()
  } catch (e) {
    saveMsg.value = `✗ ${e.message || e}`
  }
  saving.value = false
}

async function testConnection() {
  testing.value = true
  testResult.value = null
  try {
    const res = await fetch('/api/ai/test', { method: 'POST' })
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
      <a href="#/">首页</a><span class="sep">/</span><span>AI 设置</span>
    </div>

    <div class="card page-head pad-lg stagger">
      <div class="meta-row">
        <span class="chip">⚙️ 服务端设置</span>
        <span class="chip">{{ source === 'database' ? '来源：设置页' : source === 'environment' ? '来源：环境变量' : '未配置' }}</span>
        <span v-if="apiKeySet" class="chip c-ok">密钥已配置</span>
      </div>
      <h1 class="t-page">🤖 AI 设置</h1>
      <p class="summary t-body c-muted">
        配置编程题的 AI 代码评价后端。支持本地 Ollama 与 OpenAI 兼容 API；
        也可以完全不配置，编程题仍可运行测试判分。配置保存在本地数据库（data/rustway.db），
        密钥永不明文回传。
      </p>
    </div>

    <div class="card info-card pad-lg stagger" style="animation-delay: 60ms; max-width: 720px">
      <div v-if="loading" class="loading">加载配置…</div>
      <template v-else>
        <div class="form-row">
          <label>评价后端</label>
          <select v-model="backend" class="form-input">
            <option value="ollama">本地 Ollama</option>
            <option value="openai">OpenAI 兼容 API</option>
          </select>
          <p class="form-hint">ollama 默认地址 http://127.0.0.1:11434，无需密钥。</p>
        </div>

        <div class="form-row">
          <label>接口地址（Base URL）</label>
          <input
            v-model="baseUrl"
            class="form-input"
            :placeholder="backend === 'ollama' ? 'http://127.0.0.1:11434' : 'https://api.openai.com/v1'"
          />
          <p class="form-hint">仅支持 http(s) 地址；留空使用所选后端的默认值。</p>
        </div>

        <div class="form-row">
          <label>模型名称</label>
          <input
            v-model="model"
            class="form-input"
            :placeholder="backend === 'ollama' ? 'qwen2.5-coder:7b' : 'gpt-4o-mini'"
          />
        </div>

        <div class="form-row">
          <label>API 密钥{{ apiKeySet ? '（已配置）' : '（可选）' }}</label>
          <input
            v-model="apiKey"
            type="password"
            class="form-input"
            :placeholder="apiKeySet ? '已配置——留空表示不修改；输入 __CLEAR__ 可清除' : 'sk-…'"
          />
          <p class="form-hint">密钥保存在本地数据库，不会回传到页面或写入日志。openai 后端建议配置。</p>
        </div>

        <div class="form-actions">
          <button class="btn" :disabled="saving" @click="save">{{ saving ? '保存中…' : '保存配置' }}</button>
          <button class="btn btn-solid" :disabled="testing" @click="testConnection">
            {{ testing ? '⏳ 测试中…' : '🔎 测试连接' }}
          </button>
          <span v-if="saveMsg" class="t-note" :class="saveMsg.startsWith('✓') ? 'c-ok' : 'c-bad'">{{ saveMsg }}</span>
        </div>

        <div v-if="testResult" class="test-result" :class="testResult.ok ? 'ok' : 'bad'">
          <template v-if="testResult.ok">
            <b>✓ 连接成功</b> —— 模型 {{ testResult.model }}，延迟 {{ testResult.latencyMs }}ms
          </template>
          <template v-else>
            <b>✗ 连接失败</b> —— {{ testResult.error }}
            <p class="form-hint" style="margin-top: 6px">
              排查：本地 Ollama 是否已启动（ollama serve）？模型是否已拉取（ollama pull &lt;模型名&gt;）？
              地址是否可访问？
            </p>
          </template>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.form-row { display: flex; flex-direction: column; gap: 6px; margin-bottom: 18px; }
.form-row label { font-size: 13px; font-weight: 600; color: var(--text); }
.form-input {
  height: 40px; padding: 0 14px;
  border-radius: 10px; border: 1px solid var(--border);
  background: var(--panel-soft); color: var(--text);
  font-size: 13.5px; outline: none;
  transition: border-color 0.25s, box-shadow 0.25s;
  font-family: inherit;
  width: 100%; box-sizing: border-box;
}
.form-input:focus { border-color: var(--accent); box-shadow: 0 0 0 3px rgba(249, 115, 22, 0.15); }
select.form-input { appearance: auto; }
.form-hint { font-size: 12px; color: var(--faint); line-height: 1.6; margin: 0; }
.form-actions { display: flex; align-items: center; gap: 12px; margin-top: 4px; flex-wrap: wrap; }
.test-result {
  margin-top: 18px; padding: 14px 18px; border-radius: 12px;
  font-size: 13.5px; line-height: 1.7;
  animation: fadeSlide 0.4s var(--ease);
}
.test-result.ok { border: 1px solid color-mix(in srgb, var(--ok) 50%, transparent); background: color-mix(in srgb, var(--ok) 8%, var(--panel)); }
.test-result.bad { border: 1px solid color-mix(in srgb, var(--bad) 45%, transparent); background: color-mix(in srgb, var(--bad) 7%, var(--panel)); }
</style>
