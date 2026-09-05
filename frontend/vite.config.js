import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  server: {
    port: 5173,
    proxy: {
      // 开发模式下把 API 转发给 Rust 后端（cargo run 默认 8080）
      '/api': 'http://127.0.0.1:8080'
    }
  },
  build: {
    outDir: 'dist'
  }
})
