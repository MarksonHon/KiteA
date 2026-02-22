<template>
  <div class="min-h-screen flex items-center justify-center bg-gray-50 dark:bg-gray-950 px-4">
    <div class="w-full max-w-sm card p-8 space-y-6">
      <div class="text-center">
        <h1 class="text-2xl font-bold text-primary-600 dark:text-primary-400">KiteA</h1>
        <p class="text-sm text-gray-500 mt-1">登录以继续</p>
      </div>

      <form @submit.prevent="handleLogin" class="space-y-4">
        <div>
          <label class="form-label">用户名</label>
          <input v-model="form.username" type="text" class="form-input" autocomplete="username" required />
        </div>
        <div>
          <label class="form-label">密码</label>
          <input v-model="form.password" type="password" class="form-input" autocomplete="current-password" required />
        </div>
        <button type="submit" class="btn-primary w-full" :disabled="loading">
          {{ loading ? '登录中…' : '登录' }}
        </button>
      </form>

      <p v-if="error" class="text-sm text-red-500 text-center">{{ error }}</p>
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { api } from '@/utils/api'

const router  = useRouter()
const auth    = useAuthStore()
const loading = ref(false)
const error   = ref('')
const form    = ref({ username: '', password: '' })

async function handleLogin() {
  error.value   = ''
  loading.value = true
  try {
    const res = await api.post('/api/auth/login', form.value)
    auth.setAuth(res.token, res.username, res.role)
    router.push('/dashboard')
  } catch (e) {
    error.value = e.message ?? '登录失败'
  } finally {
    loading.value = false
  }
}
</script>
