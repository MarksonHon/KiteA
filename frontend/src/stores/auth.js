import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { api } from '@/utils/api'
import router from '@/router'

export const useAuthStore = defineStore('auth', () => {
  const token    = ref(localStorage.getItem('kitea_token') || '')
  const username = ref(localStorage.getItem('kitea_username') || '')
  const role     = ref(localStorage.getItem('kitea_role') || '')

  const isAdmin = computed(() => role.value === 'admin')

  function setAuth(data) {
    token.value    = data.token
    username.value = data.username
    role.value     = data.role
    localStorage.setItem('kitea_token', data.token)
    localStorage.setItem('kitea_username', data.username)
    localStorage.setItem('kitea_role', data.role)
  }

  function clearAuth() {
    token.value = ''; username.value = ''; role.value = ''
    localStorage.removeItem('kitea_token')
    localStorage.removeItem('kitea_username')
    localStorage.removeItem('kitea_role')
  }

  async function login(username, password) {
    const r = await api.post('/api/auth/login', { username, password })
    setAuth(r)   // api.js already unwraps json.data
  }

  async function logout() {
    clearAuth()
    router.push('/login')
  }

  return { token, username, role, isAdmin, setAuth, clearAuth, login, logout }
})
