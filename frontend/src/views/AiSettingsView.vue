<script setup>
/**
 * AI 设置页：查看 / 编辑 / 保存 AI 评价后端配置。
 * 配置来源优先级：设置页保存到 SQLite 的配置 > 服务端环境变量。
 * 安全契约：密钥只提交给本机服务端入库，页面上永不回显明文；
 * 地址虽可在此填写，但服务端会做 SSRF 校验（环回允许，私网/元数据地址拒绝）。
 */
import { ref, onMounted } from 'vue'

// 服务端返回的当前生效配置（只读展示）
const backend = ref('')
const baseUrl = ref('')
const model = ref('')
const apiKeySet = ref(false)
const source = ref('none')

// 编辑表单（默认从当前配置带入）
const form = ref({ backend: 'ollama', baseUrl: '', model: '', apiKey: '', clearKey: false })

const loading = ref(true)
const loadMsg = ref('')
const saving = ref(false)
const saveMsg = ref(null) // { ok, text }
const testing = ref(false)
const testResult = ref(null) // { ok, latencyMs? , error? }

const sourceLabel = { database: '来源：设置页（SQLite）', environment: '来源：环境变量', none: '未配置' }

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
    // 表单从当前生效配置带入，方便增量修改
    form.value.backend = d.backend || 'ollama'
    form.value.baseUrl = d.baseUrl || ''
    form.value.model = d.model || ''
    form.value.apiKey = ''
    form.value.clearKey = false
  } catch (e) {
    loadMsg.value = String(e.message || e)
  }
  loading.value = false
}
onMounted(load)

async function save() {
  saving.value = true
  saveMsg.value = null
  try {
    const body = {
      backend: form.value.backend,
      base_url: form.value.baseUrl.trim(),
      model: form.value.model.trim(),
      clear_api_key: form.value.clearKey,
    }
    // 留空 = 保持原密钥不变（服务端按「未提交新密钥」处理）
    if (form.value.apiKey.trim()) body.api_key = form.value.apiKey.trim()
    const res = await fetch('/api/ai/config', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    })
    const d = await res.json()
    if (!res.ok) {
      saveMsg.value = { ok: false, text: d.message || `保存失败（HTTP ${res.status}）` }
    } else {
      saveMsg.value = { ok: true, text: '✓ 已保存到本机 SQLite，立即生效' }
      await load()
    }
  } catch (e) {
    saveMsg.value = { ok: false, text: String(e.message || e) }
  }
  saving.value = false
}

async function testConnection() {
  testing.value = true
  testResult.value = null
  try {
    const res = await fetch('/api/ai/latency', { method: 'POST' })
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
        <span class="chip" :class="{ 'c-ok': source !== 'none' }">{{ sourceLabel[source] || '未配置' }}</span>
        <span v-if="apiKeySet" class="chip c-ok">密钥已配置</span>
      </div>
      <h1 class="t-page">🤖 AI 评价设置</h1>
      <p class="summary t-body c-muted">
        为编程题与主观题提供 AI 评价/批改。在下方保存配置（存本机 SQLite，优先于环境变量），
        或使用环境变量 RUSTWAY_AI_BACKEND / RUSTWAY_AI_BASE_URL / RUSTWAY_AI_MODEL / RUSTWAY_AI_API_KEY。
        不配置也不影响编程题的测试判分。
      </p>
    </div>

    <div class="card info-card pad-lg stagger" style="animation-delay: 60ms; max-width: 720px">
      <div v-if="loading" class="loading">加载配置…</div>
      <template v-else>
        <div class="cfg-row"><span class="cfg-label">生效后端</span><b>{{ backend || '—' }}</b></div>
        <div class="cfg-row"><span class="cfg-label">接口地址</span><code>{{ baseUrl || '—' }}</code></div>
        <div class="cfg-row"><span class="cfg-label">模型</span><b>{{ model || '—' }}</b></div>
        <div class="cfg-row"><span class="cfg-label">密钥</span><b>{{ apiKeySet ? '已配置' : '未配置' }}</b></div>
        <p v-if="loadMsg" class="t-note c-bad">{{ loadMsg }}</p>

        <h2 class="form-title">修改配置</h2>
        <div class="field">
          <label class="field-label" for="ai-backend">后端</label>
          <select id="ai-backend" v-model="form.backend" class="field-input">
            <option value="ollama">ollama（本地，默认 http://127.0.0.1:11434）</option>
            <option value="openai">openai 兼容 API（需地址 + 模型 + 密钥）</option>
          </select>
        </div>
        <div class="field">
          <label class="field-label" for="ai-baseurl">接口地址 base_url</label>
          <input
            id="ai-baseurl" v-model="form.baseUrl" class="field-input"
            :placeholder="form.backend === 'ollama' ? 'http://127.0.0.1:11434（可留空）' : 'https://api.openai.com/v1（必填）'"
            autocomplete="off" spellcheck="false"
          >
        </div>
        <div class="field">
          <label class="field-label" for="ai-model">模型</label>
          <input
            id="ai-model" v-model="form.model" class="field-input"
            :placeholder="form.backend === 'ollama' ? 'qwen2.5-coder:7b（可留空）' : 'gpt-4o-mini（必填）'"
            autocomplete="off" spellcheck="false"
          >
        </div>
        <div class="field">
          <label class="field-label" for="ai-key">API 密钥（仅 openai 兼容后端）</label>
          <input
            id="ai-key" v-model="form.apiKey" class="field-input" type="password"
            :placeholder="apiKeySet ? '已配置 —— 留空保持不变' : '未配置（本地 ollama 无需密钥）'"
            autocomplete="new-password"
          >
        </div>
        <label class="check-row" for="ai-clearkey">
          <input id="ai-clearkey" v-model="form.clearKey" type="checkbox">
          <span>清除已保存的密钥</span>
        </label>

        <div class="form-actions">
          <button class="btn btn-solid" :disabled="saving" @click="save">
            {{ saving ? '⏳ 保存中…' : '💾 保存配置' }}
          </button>
          <button class="btn btn-ghost" :disabled="testing || source === 'none'" @click="testConnection">
            {{ testing ? '⏳ 测试中…' : '🔎 测试连接' }}
          </button>
        </div>
        <p v-if="saveMsg" class="t-note" :class="saveMsg.ok ? 'c-ok' : 'c-bad'">{{ saveMsg.text }}</p>
        <p class="form-hint">
          安全说明：密钥仅存本机 SQLite 且接口永不回传明文；地址经服务端 SSRF 校验
          （环回允许，私网 / 云元数据地址拒绝）。
        </p>

        <div v-if="testResult" class="test-result" :class="testResult.ok ? 'ok' : 'bad'">
          <template v-if="testResult.ok">
            <b>✓ 连接成功</b> —— 模型 {{ testResult.model }}，延迟 {{ testResult.latencyMs }}ms
          </template>
          <template v-else>
            <b>✗ 连接失败</b> —— {{ testResult.error }}
            <p class="form-hint" style="margin-top: 6px">
              排查：本地 Ollama 是否已启动（ollama serve）？模型是否已拉取（ollama pull &lt;模型名&gt;）？
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
.form-title { font-size: 15px; margin: 22px 0 4px; color: var(--text); }
.field { display: flex; flex-direction: column; gap: 6px; margin-top: 12px; }
.field-label { font-size: 12.5px; color: var(--faint); }
.field-input {
  height: 38px; padding: 0 12px; width: 100%; box-sizing: border-box;
  border-radius: 10px; border: 1px solid var(--border);
  background: var(--panel-soft); color: var(--text);
  font-size: 13px; outline: none;
  transition: border-color 0.25s, box-shadow 0.25s;
}
.field-input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px rgba(249, 115, 22, 0.18);
}
.check-row {
  display: flex; align-items: center; gap: 8px; margin-top: 12px;
  font-size: 12.5px; color: var(--muted); cursor: pointer; user-select: none;
}
.check-row input { accent-color: var(--accent); }
.test-result {
  margin-top: 18px; padding: 14px 18px; border-radius: 12px;
  font-size: 13.5px; line-height: 1.7;
  animation: fadeSlide 0.4s var(--ease);
}
.test-result.ok { border: 1px solid color-mix(in srgb, var(--ok) 50%, transparent); background: color-mix(in srgb, var(--ok) 8%, var(--panel)); }
.test-result.bad { border: 1px solid color-mix(in srgb, var(--bad) 45%, transparent); background: color-mix(in srgb, var(--bad) 7%, var(--panel)); }
</style>
