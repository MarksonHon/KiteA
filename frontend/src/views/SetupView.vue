<template>
  <div class="min-h-screen flex items-center justify-center bg-gray-50 dark:bg-gray-950 px-4">
    <div class="w-full max-w-sm card p-8 space-y-6">
      <div class="text-center">
        <h1 class="text-2xl font-bold text-primary-600 dark:text-primary-400">KiteA 初始化</h1>
        <p class="text-sm text-gray-500 mt-1">创建管理员账户</p>
      </div>

      <form @submit.prevent="handleSetup" class="space-y-4">
        <div>
          <label class="form-label">管理员用户名</label>
          <input v-model="form.username" type="text" class="form-input" required />
        </div>
        <div>
          <label class="form-label">密码</label>
          <input v-model="form.password" type="password" class="form-input" required />
        </div>
        <div>
          <label class="form-label">确认密码</label>
          <input v-model="form.confirm" type="password" class="form-input" required />
        </div>
        <button type="submit" class="btn-primary w-full" :disabled="loading">
          {{ loading ? '初始化中…' : '初始化' }}
        </button>
      </form>

      <p v-if="error" class="text-sm text-red-500 text-center">{{ error }}</p>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { api } from '@/utils/api'
import { invalidateSetupCache } from '@/router/index'

const router  = useRouter()
const auth    = useAuthStore()
const loading = ref(false)
const error   = ref('')
const form    = ref({ username: '', password: '', confirm: '' })

// 若用户已存在（直接访问 /setup），跳回登录页
onMounted(async () => {
  try {
    const res = await fetch('/api/setup/needed')
    const json = await res.json()
    if (!json.data?.needed) router.replace('/login')
  } catch {}
})

async function handleSetup() {
  error.value = ''
  if (form.value.password !== form.value.confirm) { error.value = '两次密码不一致'; return }
  loading.value = true
  try {
    const res = await api.post('/api/auth/setup', { username: form.value.username, password: form.value.password })
    invalidateSetupCache()   // 让路由守卫下次重新检测
    auth.setAuth(res.token, res.username, res.role)
    router.push('/dashboard')
  } catch (e) {
    error.value = e.message ?? '初始化失败'
  } finally {
    loading.value = false
  }
}
</script>
