import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
  plugins: [vue(), tailwindcss()],
  server: {
    host: '0.0.0.0',
    port: 1420,
    strictPort: true
  },
  preview: {
    host: '0.0.0.0',
    port: 4174,
    strictPort: false
  }
});
