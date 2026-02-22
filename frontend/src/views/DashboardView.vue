<template>
  <div class="space-y-6">
    <h2 class="text-xl font-semibold">仪表盘</h2>

    <!-- Status card -->
    <div class="card p-6 flex items-center gap-6">
      <div class="flex flex-col items-center gap-2">
        <div :class="['w-16 h-16 rounded-full flex items-center justify-center text-3xl font-bold transition-colors',
          status.running ? 'bg-green-100 dark:bg-green-900/30 text-green-600' : 'bg-gray-100 dark:bg-gray-800 text-gray-400']">
          {{ status.running ? '▶' : '■' }}
        </div>
        <span :class="['text-sm font-medium', status.running ? 'text-green-600' : 'text-gray-400']">
          {{ status.running ? '运行中' : '已停止' }}
        </span>
      </div>

      <div class="flex-1 space-y-1">
        <div class="text-sm text-gray-500">监听地址 <span class="text-gray-900 dark:text-gray-100 font-mono">{{ status.listen || '—' }}</span></div>
        <div class="text-sm text-gray-500">当前节点 <span class="text-gray-900 dark:text-gray-100">{{ activeName || '（未选择）' }}</span></div>
        <div class="text-sm text-gray-500">本地端口 <span class="text-gray-900 dark:text-gray-100 font-mono">{{ settings.local_port || 1080 }}</span></div>
        <div class="text-sm text-gray-500">
          TUN 模式
          <span :class="['ml-1 font-medium', status.tun_running ? 'text-green-600' : 'text-gray-400']">
            {{ status.tun_running ? '● 运行中' : '○ 未启用' }}
          </span>
        </div>
      </div>

      <div class="flex flex-col gap-2">
        <button v-if="!status.running" class="btn-primary" :disabled="loading || !activeNodes.length" @click="startProxy">
          {{ loading ? '启动中…' : '启动代理' }}
        </button>
        <button v-else class="btn-danger" :disabled="loading" @click="stopProxy">
          {{ loading ? '停止中…' : '停止代理' }}
        </button>
        <button class="btn-secondary text-xs" @click="refresh">刷新状态</button>
      </div>
    </div>

    <!-- Stats row -->
    <div class="grid grid-cols-2 sm:grid-cols-4 gap-4">
      <StatCard label="启用节点" :value="activeNodes.length" />
      <StatCard label="节点总数" :value="stats.nodes" />
      <StatCard label="订阅数量" :value="stats.subscriptions" />
      <StatCard label="用户数量" :value="stats.users" icon="" />
    </div>

    <!-- Quick selectors -->
    <div class="card p-4 space-y-3">
      <h3 class="font-medium text-sm">快速启动（选择节点）</h3>
      <div v-if="activeNodes.length === 0" class="text-sm text-gray-400">暂无已启用节点，请前往节点管理添加。</div>
      <div v-else class="flex flex-wrap gap-2">
        <button
          v-for="n in activeNodes.slice(0, 12)"
          :key="n.id"
          :class="['tag-btn', selectedId === n.id ? 'tag-btn-active' : '']"
          @click="selectedId = n.id"
        >
          <span class="protocol-badge">{{ n.protocol }}</span>
          {{ n.name }}
        </button>
      </div>
    </div>

    <!-- Error msg -->
    <p v-if="error" class="text-sm text-red-500">{{ error }}</p>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
import { api } from '@/utils/api'
import { useToastStore } from '@/stores/toast'
import { generateShoesConfig } from '@/utils/configGen'
import StatCard from '@/components/StatCard.vue'

const toast      = useToastStore()
const loading    = ref(false)
const error      = ref('')
const status     = ref({ running: false, listen: '' })
const settings   = ref({ local_port: 1080 })
const activeNodes = ref([])
const stats      = ref({ nodes: 0, subscriptions: 0, users: 0 })
const selectedId = ref(null)

const activeName = computed(() => activeNodes.value.find(n => n.id === selectedId.value)?.name)

async function refresh() {
  try {
    const [st, set, nodes, subs] = await Promise.all([
      api.get('/api/proxy/status'),
      api.get('/api/settings'),
      api.get('/api/nodes?enabled=true'),
      api.get('/api/subscriptions'),
    ])
    status.value   = st
    settings.value = set
    activeNodes.value = nodes.nodes ?? nodes
    stats.value.nodes         = nodes.total ?? nodes.length
    stats.value.subscriptions = subs.length ?? 0
  } catch (e) {
    error.value = e.message
  }
}

async function startProxy() {
  error.value = ''
  loading.value = true
  try {
    const nodes = selectedId.value
      ? activeNodes.value.filter(n => n.id === selectedId.value)
      : activeNodes.value

    const yaml = generateShoesConfig(nodes, settings.value.local_port ?? 1080)
    await api.post('/api/proxy/start', { yaml })
    toast.success('代理已启动')
    await refresh()
  } catch (e) {
    error.value = e.message
    toast.error('启动失败：' + e.message)
  } finally {
    loading.value = false
  }
}

async function stopProxy() {
  loading.value = true
  try {
    await api.post('/api/proxy/stop', {})
    toast.success('代理已停止')
    await refresh()
  } catch (e) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}

onMounted(refresh)
</script>

<style scoped>
.tag-btn {
  @apply flex items-center gap-1.5 px-3 py-1.5 rounded-full border border-gray-200 dark:border-gray-700 text-xs
         hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors cursor-pointer;
}
.tag-btn-active {
  @apply border-primary-500 bg-primary-50 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300;
}
.protocol-badge {
  @apply bg-gray-100 dark:bg-gray-700 px-1 py-0.5 rounded text-[10px] font-mono uppercase;
}
</style>
