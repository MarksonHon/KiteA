<template>
  <div class="space-y-4">
    <div class="flex items-center justify-between">
      <h2 class="text-xl font-semibold">订阅管理</h2>
      <button class="btn-primary" @click="showAdd = true">添加订阅</button>
    </div>

    <div class="space-y-3">
      <div v-if="subs.length === 0" class="card p-8 text-center text-gray-400 text-sm">暂无订阅</div>
      <div v-for="s in subs" :key="s.id" class="card p-4 flex items-center gap-4">
        <div class="flex-1 min-w-0">
          <div class="font-medium truncate">{{ s.name }}</div>
          <div class="text-xs text-gray-500 font-mono truncate">{{ s.url }}</div>
          <div class="text-xs text-gray-400 mt-1">
            上次更新：{{ s.last_update ? new Date(s.last_update).toLocaleString() : '未更新' }}
          </div>
        </div>
        <div class="flex gap-2 shrink-0">
          <button class="btn-secondary text-xs" :disabled="updating === s.id" @click="updateSub(s)">
            {{ updating === s.id ? '更新中…' : '更新' }}
          </button>
          <button class="btn-danger-secondary text-xs" @click="deleteSub(s.id)">删除</button>
        </div>
      </div>
    </div>

    <!-- Add modal -->
    <Teleport to="body">
      <div v-if="showAdd" class="modal-overlay" @click.self="showAdd = false">
        <div class="modal-box space-y-4">
          <h3 class="font-semibold">添加订阅</h3>
          <div>
            <label class="form-label">名称</label>
            <input v-model="addForm.name" type="text" class="form-input w-full" placeholder="机场名称" />
          </div>
          <div>
            <label class="form-label">订阅 URL</label>
            <input v-model="addForm.url" type="url" class="form-input w-full" placeholder="https://…" />
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
import { parseSubscriptionContent } from '@/utils/nodeParser'

const toast   = useToastStore()
const subs    = ref([])
const showAdd = ref(false)
const adding  = ref(false)
const updating = ref(null)
const addForm = ref({ name: '', url: '' })

async function load() {
  try { subs.value = await api.get('/api/subscriptions') } catch (e) { toast.error(e.message) }
}

async function doAdd() {
  if (!addForm.value.name || !addForm.value.url) return
  adding.value = true
  try {
    await api.post('/api/subscriptions', addForm.value)
    toast.success('订阅已添加')
    showAdd.value = false
    addForm.value = { name: '', url: '' }
    await load()
  } catch (e) { toast.error(e.message) }
  finally { adding.value = false }
}

async function updateSub(s) {
  updating.value = s.id
  try {
    // Fetch subscription content
    const resp = await fetch(s.url)
    const text = await resp.text()
    const uris = parseSubscriptionContent(text)
    if (!uris.length) throw new Error('订阅内容为空或无法解析')
    await api.post('/api/nodes/import-subscription', { group: s.name, uris })
    // Update last_update timestamp
    await api.put(`/api/subscriptions/${s.id}`, { last_update: new Date().toISOString() })
    toast.success(`已导入 ${uris.length} 个节点`)
    await load()
  } catch (e) { toast.error('更新失败：' + e.message) }
  finally { updating.value = null }
}

async function deleteSub(id) {
  if (!confirm('确认删除该订阅？')) return
  try {
    await api.delete(`/api/subscriptions/${id}`)
    subs.value = subs.value.filter(s => s.id !== id)
    toast.success('已删除')
  } catch (e) { toast.error(e.message) }
}

onMounted(load)
</script>

<style scoped>
.modal-overlay { @apply fixed inset-0 z-50 bg-black/50 flex items-center justify-center p-4; }
.modal-box     { @apply bg-white dark:bg-gray-900 rounded-xl shadow-xl p-6 w-full max-w-md; }
</style>
