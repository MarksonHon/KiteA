<template>
  <div class="space-y-4">
    <div class="flex items-center justify-between">
      <h2 class="text-xl font-semibold">节点管理</h2>
      <div class="flex gap-2">
        <button class="btn-secondary" @click="showImport = true">导入节点</button>
        <button class="btn-danger-secondary" :disabled="!selected.size" @click="deleteSelected">
          删除选中 ({{ selected.size }})
        </button>
      </div>
    </div>

    <!-- Filter bar -->
    <div class="flex flex-wrap gap-2 items-center">
      <input v-model="search" type="text" placeholder="搜索节点名称或服务器…" class="form-input w-56" />
      <select v-model="filterProto" class="form-input w-32">
        <option value="">全部协议</option>
        <option v-for="p in protocols" :key="p" :value="p">{{ p }}</option>
      </select>
      <label class="flex items-center gap-1.5 text-sm text-gray-600 dark:text-gray-400 cursor-pointer">
        <input type="checkbox" v-model="showEnabled" class="rounded" /> 仅显示已启用
      </label>
    </div>

    <!-- Table -->
    <div class="card overflow-hidden">
      <table class="w-full text-sm">
        <thead class="bg-gray-50 dark:bg-gray-800 text-xs text-gray-500 uppercase">
          <tr>
            <th class="px-3 py-2 text-left w-8">
              <input type="checkbox" :checked="allSelected" @change="toggleAll" class="rounded" />
            </th>
            <th class="px-3 py-2 text-left">名称</th>
            <th class="px-3 py-2 text-left">协议</th>
            <th class="px-3 py-2 text-left">服务器</th>
            <th class="px-3 py-2 text-left">端口</th>
            <th class="px-3 py-2 text-left">分组</th>
            <th class="px-3 py-2 text-center">启用</th>
            <th class="px-3 py-2 text-right">操作</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-gray-100 dark:divide-gray-800">
          <tr v-if="filtered.length === 0">
            <td colspan="8" class="px-3 py-8 text-center text-gray-400">暂无节点</td>
          </tr>
          <tr v-for="n in filtered" :key="n.id"
              class="hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors">
            <td class="px-3 py-2">
              <input type="checkbox" :checked="selected.has(n.id)" @change="toggleSelect(n.id)" class="rounded" />
            </td>
            <td class="px-3 py-2 font-medium max-w-[160px] truncate" :title="n.name">{{ n.name }}</td>
            <td class="px-3 py-2">
              <span class="protocol-badge">{{ n.protocol }}</span>
            </td>
            <td class="px-3 py-2 font-mono text-xs max-w-[140px] truncate" :title="n.server">{{ n.server }}</td>
            <td class="px-3 py-2 font-mono text-xs">{{ n.port }}</td>
            <td class="px-3 py-2 text-xs text-gray-500">{{ n.group || '—' }}</td>
            <td class="px-3 py-2 text-center">
              <input type="checkbox" :checked="n.enabled" @change="toggleEnable(n)" class="rounded" />
            </td>
            <td class="px-3 py-2 text-right">
              <button class="text-xs text-blue-500 hover:underline mr-2" @click="viewUri(n)">URI</button>
              <button class="text-xs text-red-500 hover:underline" @click="deleteNode(n.id)">删除</button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Import modal -->
    <Teleport to="body">
      <div v-if="showImport" class="modal-overlay" @click.self="showImport = false">
        <div class="modal-box space-y-4">
          <h3 class="font-semibold">导入节点 URI</h3>
          <p class="text-xs text-gray-500">每行一个 URI，支持 vmess:// vless:// ss:// trojan:// hy2:// tuic://</p>
          <textarea
            v-model="importText"
            rows="8"
            class="form-input font-mono text-xs w-full resize-none"
            placeholder="vmess://...&#10;vless://...&#10;ss://..."
          />
          <div class="flex justify-end gap-2">
            <button class="btn-secondary" @click="showImport = false">取消</button>
            <button class="btn-primary" :disabled="importing" @click="doImport">
              {{ importing ? '导入中…' : '确认导入' }}
            </button>
          </div>
          <p v-if="importResult" class="text-xs text-green-600">{{ importResult }}</p>
        </div>
      </div>
    </Teleport>

    <!-- URI viewer modal -->
    <Teleport to="body">
      <div v-if="viewingNode" class="modal-overlay" @click.self="viewingNode = null">
        <div class="modal-box space-y-3">
          <h3 class="font-semibold">节点 URI</h3>
          <textarea :value="viewingNode.uri" readonly rows="4" class="form-input font-mono text-xs w-full resize-none" />
          <div class="flex justify-end">
            <button class="btn-secondary" @click="viewingNode = null">关闭</button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { api } from '@/utils/api'
import { useToastStore } from '@/stores/toast'

const toast       = useToastStore()
const nodes       = ref([])
const search      = ref('')
const filterProto = ref('')
const showEnabled = ref(false)
const selected    = ref(new Set())
const showImport  = ref(false)
const importText  = ref('')
const importing   = ref(false)
const importResult = ref('')
const viewingNode = ref(null)

const protocols = computed(() => [...new Set(nodes.value.map(n => n.protocol))].sort())

const filtered = computed(() =>
  nodes.value.filter(n => {
    if (showEnabled.value && !n.enabled) return false
    if (filterProto.value && n.protocol !== filterProto.value) return false
    const q = search.value.toLowerCase()
    return !q || n.name.toLowerCase().includes(q) || n.server.toLowerCase().includes(q)
  })
)

const allSelected = computed(() => filtered.value.length > 0 && filtered.value.every(n => selected.value.has(n.id)))

function toggleAll() {
  if (allSelected.value) filtered.value.forEach(n => selected.value.delete(n.id))
  else                   filtered.value.forEach(n => selected.value.add(n.id))
}
function toggleSelect(id) {
  selected.value.has(id) ? selected.value.delete(id) : selected.value.add(id)
}

async function load() {
  try {
    const res = await api.get('/api/nodes')
    nodes.value = res.nodes ?? res
  } catch (e) {
    toast.error(e.message)
  }
}

async function toggleEnable(node) {
  try {
    await api.put(`/api/nodes/${node.id}`, { enabled: !node.enabled })
    node.enabled = !node.enabled
  } catch (e) {
    toast.error(e.message)
  }
}

async function deleteNode(id) {
  if (!confirm('确认删除该节点？')) return
  try {
    await api.delete(`/api/nodes/${id}`)
    nodes.value = nodes.value.filter(n => n.id !== id)
    selected.value.delete(id)
    toast.success('已删除')
  } catch (e) {
    toast.error(e.message)
  }
}

async function deleteSelected() {
  if (!confirm(`确认删除选中的 ${selected.value.size} 个节点？`)) return
  for (const id of selected.value) {
    try { await api.delete(`/api/nodes/${id}`) } catch {}
  }
  selected.value.clear()
  await load()
  toast.success('已批量删除')
}

async function doImport() {
  const uris = importText.value.split('\n').map(s => s.trim()).filter(Boolean)
  if (!uris.length) return
  importing.value = true
  importResult.value = ''
  let ok = 0, fail = 0
  for (const uri of uris) {
    try {
      await api.post('/api/nodes/import', { uri })
      ok++
    } catch { fail++ }
  }
  importResult.value = `导入完成：成功 ${ok}，失败 ${fail}`
  await load()
  importing.value = false
  if (ok > 0) importText.value = ''
}

function viewUri(n) { viewingNode.value = n }

onMounted(load)
</script>

<style scoped>
.protocol-badge {
  @apply inline-block px-1.5 py-0.5 rounded bg-gray-100 dark:bg-gray-700 text-[10px] font-mono uppercase;
}
.modal-overlay {
  @apply fixed inset-0 z-50 bg-black/50 flex items-center justify-center p-4;
}
.modal-box {
  @apply bg-white dark:bg-gray-900 rounded-xl shadow-xl p-6 w-full max-w-lg;
}
</style>
