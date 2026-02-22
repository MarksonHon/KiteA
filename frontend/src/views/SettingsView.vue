<template>
  <div class="space-y-4 max-w-lg">
    <h2 class="text-xl font-semibold">系统设置</h2>

    <div class="card p-6 space-y-4">
      <div>
        <label class="form-label">本地代理端口（mixed HTTP/SOCKS5）</label>
        <input v-model.number="form.local_port" type="number" min="1" max="65535" class="form-input w-40" />
      </div>
      <div>
        <label class="form-label">shoes 可执行文件路径</label>
        <input v-model="form.shoes_bin" type="text" class="form-input w-full" placeholder="shoes（留空则从 PATH 查找）" />
      </div>
      <div>
        <label class="form-label">DNS 服务器（可选）</label>
        <input v-model="form.dns_server" type="text" class="form-input w-full" placeholder="8.8.8.8" />
      </div>
      <div class="pt-2">
        <button class="btn-primary" :disabled="saving" @click="save">
          {{ saving ? '保存中…' : '保存设置' }}
        </button>
      </div>
    </div>

    <p v-if="saved" class="text-sm text-green-500">✓ 设置已保存</p>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { api } from '@/utils/api'
import { useToastStore } from '@/stores/toast'

const toast = useToastStore()
const saving = ref(false)
const saved  = ref(false)
const form   = ref({ local_port: 1080, shoes_bin: '', dns_server: '' })

async function load() {
  try { Object.assign(form.value, await api.get('/api/settings')) } catch (e) { toast.error(e.message) }
}
async function save() {
  saving.value = true; saved.value = false
  try {
    await api.put('/api/settings', form.value)
    saved.value = true
    toast.success('设置已保存')
    setTimeout(() => { saved.value = false }, 3000)
  } catch (e) { toast.error(e.message) }
  finally { saving.value = false }
}

onMounted(load)
</script>
