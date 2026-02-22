<template>
  <div class="space-y-4">
    <div class="flex items-center justify-between">
      <h2 class="text-xl font-semibold">用户管理</h2>
      <button class="btn-primary" @click="showAdd = true">添加用户</button>
    </div>

    <div class="card overflow-hidden">
      <table class="w-full text-sm">
        <thead class="bg-gray-50 dark:bg-gray-800 text-xs text-gray-500 uppercase">
          <tr>
            <th class="px-4 py-2 text-left">用户名</th>
            <th class="px-4 py-2 text-left">角色</th>
            <th class="px-4 py-2 text-left">创建时间</th>
            <th class="px-4 py-2 text-right">操作</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-100 dark:divide-gray-800">
          <tr v-if="users.length === 0">
            <td colspan="4" class="px-4 py-8 text-center text-gray-400">暂无用户</td>
          </tr>
          <tr v-for="u in users" :key="u.id" class="hover:bg-gray-50 dark:hover:bg-gray-800/50">
            <td class="px-4 py-3 font-medium">{{ u.username }}</td>
            <td class="px-4 py-3">
              <span :class="['badge', u.role === 'admin' ? 'badge-admin' : 'badge-user']">{{ u.role }}</span>
            </td>
            <td class="px-4 py-3 text-xs text-gray-500">{{ new Date(u.created_at).toLocaleString() }}</td>
            <td class="px-4 py-3 text-right">
              <button class="text-xs text-red-500 hover:underline" @click="deleteUser(u)">删除</button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Add modal -->
    <Teleport to="body">
      <div v-if="showAdd" class="modal-overlay" @click.self="showAdd = false">
        <div class="modal-box space-y-4">
          <h3 class="font-semibold">添加用户</h3>
          <div>
            <label class="form-label">用户名</label>
            <input v-model="addForm.username" type="text" class="form-input w-full" />
          </div>
          <div>
            <label class="form-label">密码</label>
            <input v-model="addForm.password" type="password" class="form-input w-full" />
          </div>
          <div>
            <label class="form-label">角色</label>
            <select v-model="addForm.role" class="form-input w-full">
              <option value="user">user</option>
              <option value="admin">admin</option>
            </select>
          </div>
          <div class="flex justify-end gap-2">
            <button class="btn-secondary" @click="showAdd = false">取消</button>
            <button class="btn-primary" :disabled="adding" @click="doAdd">{{ adding ? '添加中…' : '确认' }}</button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'
import { api } from '@/utils/api'
import { useToastStore } from '@/stores/toast'
import { useAuthStore } from '@/stores/auth'

const toast   = useToastStore()
const auth    = useAuthStore()
const users   = ref([])
const showAdd = ref(false)
const adding  = ref(false)
const addForm = ref({ username: '', password: '', role: 'user' })

async function load() {
  try { users.value = await api.get('/api/users') } catch (e) { toast.error(e.message) }
}
async function doAdd() {
  adding.value = true
  try {
    await api.post('/api/users', addForm.value)
    toast.success('用户已添加')
    showAdd.value = false
    addForm.value = { username: '', password: '', role: 'user' }
    await load()
  } catch (e) { toast.error(e.message) }
  finally { adding.value = false }
}
async function deleteUser(u) {
  if (u.username === auth.username) { toast.error('不能删除自己'); return }
  if (!confirm(`确认删除用户 "${u.username}"？`)) return
  try {
    await api.delete(`/api/users/${u.id}`)
    users.value = users.value.filter(x => x.id !== u.id)
    toast.success('已删除')
  } catch (e) { toast.error(e.message) }
}

onMounted(load)
</script>

<style scoped>
.badge      { @apply inline-block px-2 py-0.5 rounded text-xs font-medium; }
.badge-admin { @apply bg-primary-100 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300; }
.badge-user  { @apply bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300; }
.modal-overlay { @apply fixed inset-0 z-50 bg-black/50 flex items-center justify-center p-4; }
.modal-box { @apply bg-white dark:bg-gray-900 rounded-xl shadow-xl p-6 w-full max-w-sm; }
</style>
