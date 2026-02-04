import { fileURLToPath, URL } from 'node:url'

import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vueDevTools from 'vite-plugin-vue-devtools'

// https://vite.dev/config/
export default defineConfig({
    server: {
        host: '0.0.0.0',
        https: {
            key: './certs/localhost-key.pem',
            cert: './certs/localhost-cert.pem',
        },
        proxy: {
            '/mc': {
                target: 'http://127.0.0.1:9855',
                changeOrigin: true,
                rewrite: (path) => path.replace(/^\/mc/, '')
            },
            '/api': {
                target: 'http://127.0.0.1:9855',
                changeOrigin: true,
                rewrite: (path) => path.replace(/^\/api/, 'api')
            }
        }
    },
  plugins: [
    vue(),
    vueDevTools(),
  ],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url))
    },
  },
})
