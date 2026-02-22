<template>
  <div class="space-y-4">
    <!-- Header -->
    <div class="flex flex-wrap items-center gap-2">
      <h2 class="text-xl font-semibold flex-1">节点管理</h2>
      <button class="btn-primary"          @click="openAdd">手动添加</button>
      <button class="btn-secondary"        @click="showImport = true">URI 导入</button>
      <button class="btn-secondary"        @click="doExport">导出节点</button>
      <button class="btn-danger-secondary" :disabled="!selected.size" @click="deleteSelected">
        删除选中 ({{ selected.size }})
      </button>
    </div>

    <!-- Filter bar -->
    <div class="flex flex-wrap gap-2 items-center">
      <input v-model="search" type="text" placeholder="搜索名称/服务器…" class="form-input w-52" />
      <select v-model="filterProto" class="form-input w-36">
        <option value="">全部协议</option>
        <option v-for="p in protocols" :key="p" :value="p">{{ p }}</option>
      </select>
      <label class="flex items-center gap-1.5 text-sm cursor-pointer select-none">
        <input type="checkbox" v-model="showEnabled" class="rounded" /> 仅启用
      </label>
    </div>

    <!-- Table -->
    <div class="card overflow-hidden !p-0">
      <table class="w-full text-sm">
        <thead class="bg-gray-50 dark:bg-gray-800 text-xs text-gray-500 uppercase">
          <tr>
            <th class="px-3 py-2 w-8"><input type="checkbox" :checked="allSelected" @change="toggleAll" class="rounded" /></th>
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
            <td colspan="8" class="px-3 py-10 text-center text-gray-400 text-sm">暂无节点</td>
          </tr>
          <tr v-for="n in filtered" :key="n.id"
              class="hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors">
            <td class="px-3 py-2"><input type="checkbox" :checked="selected.has(n.id)" @change="toggleSelect(n.id)" class="rounded" /></td>
            <td class="px-3 py-2 font-medium max-w-[160px] truncate" :title="n.name">{{ n.name }}</td>
            <td class="px-3 py-2"><span class="protocol-badge">{{ n.protocol }}</span></td>
            <td class="px-3 py-2 font-mono text-xs max-w-[140px] truncate" :title="n.server">{{ n.server }}</td>
            <td class="px-3 py-2 font-mono text-xs">{{ n.port }}</td>
            <td class="px-3 py-2 text-xs text-gray-500 truncate max-w-[80px]">{{ n.group || '—' }}</td>
            <td class="px-3 py-2 text-center">
              <input type="checkbox" :checked="n.enabled" @change="toggleEnable(n)" class="rounded" />
            </td>
            <td class="px-3 py-2 text-right whitespace-nowrap">
              <button class="text-xs text-blue-500 hover:underline mr-2" @click="viewUri(n)">URI</button>
              <button class="text-xs text-red-500 hover:underline" @click="deleteNode(n.id)">删除</button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- ── 手动添加 Modal ── -->
    <Teleport to="body">
      <div v-if="showAdd" class="modal-overlay" @click.self="showAdd = false">
        <div class="modal-box max-w-2xl space-y-4 overflow-y-auto max-h-[90vh]">
          <h3 class="font-semibold text-lg">手动添加节点</h3>

          <!-- Common fields -->
          <div class="grid grid-cols-2 gap-3">
            <div class="col-span-2">
              <label class="form-label">节点名称</label>
              <input v-model="addForm.name" type="text" class="form-input w-full" placeholder="My Node" />
            </div>
            <div>
              <label class="form-label">协议</label>
              <select v-model="addForm.protocol" class="form-input w-full">
                <option v-for="p in PROTOCOLS" :key="p.value" :value="p.value">{{ p.label }}</option>
              </select>
            </div>
            <div></div>
            <div>
              <label class="form-label">服务器地址</label>
              <input v-model="addForm.server" type="text" class="form-input w-full" placeholder="example.com" />
            </div>
            <div>
              <label class="form-label">端口</label>
              <input v-model.number="addForm.port" type="number" min="1" max="65535" class="form-input w-full" placeholder="443" />
            </div>
          </div>

          <!-- Protocol-specific fields -->
          <div class="grid grid-cols-2 gap-3 border-t border-gray-200 dark:border-gray-700 pt-3">

            <!-- vmess -->
            <template v-if="addForm.protocol === 'vmess'">
              <div class="col-span-2">
                <label class="form-label">UUID</label>
                <input v-model="addForm.uuid" type="text" class="form-input w-full font-mono" placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx" />
              </div>
              <div>
                <label class="form-label">加密</label>
                <select v-model="addForm.cipher" class="form-input w-full">
                  <option value="auto">auto</option>
                  <option value="aes-128-gcm">aes-128-gcm</option>
                  <option value="chacha20-poly1305">chacha20-poly1305</option>
                  <option value="none">none</option>
                </select>
              </div>
              <div>
                <label class="form-label">TLS</label>
                <select v-model="addForm.tls" class="form-input w-full">
                  <option value="">无</option>
                  <option value="tls">TLS</option>
                </select>
              </div>
              <div>
                <label class="form-label">传输层</label>
                <select v-model="addForm.transport" class="form-input w-full">
                  <option value="tcp">TCP</option>
                  <option value="ws">WebSocket</option>
                  <option value="h2">HTTP/2</option>
                  <option value="grpc">gRPC</option>
                </select>
              </div>
              <div v-if="addForm.tls === 'tls'">
                <label class="form-label">SNI</label>
                <input v-model="addForm.sni" type="text" class="form-input w-full" />
              </div>
              <div v-if="addForm.transport !== 'tcp' && addForm.transport !== 'grpc'">
                <label class="form-label">Path</label>
                <input v-model="addForm.path" type="text" class="form-input w-full" placeholder="/" />
              </div>
              <div v-if="addForm.transport === 'ws' || addForm.transport === 'h2'">
                <label class="form-label">Host</label>
                <input v-model="addForm.host" type="text" class="form-input w-full" />
              </div>
            </template>

            <!-- vless -->
            <template v-if="addForm.protocol === 'vless'">
              <div class="col-span-2">
                <label class="form-label">UUID</label>
                <input v-model="addForm.uuid" type="text" class="form-input w-full font-mono" placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx" />
              </div>
              <div>
                <label class="form-label">安全层</label>
                <select v-model="addForm.security" class="form-input w-full">
                  <option value="none">无</option>
                  <option value="tls">TLS</option>
                  <option value="reality">Reality</option>
                </select>
              </div>
              <div>
                <label class="form-label">传输层</label>
                <select v-model="addForm.transport" class="form-input w-full">
                  <option value="tcp">TCP</option>
                  <option value="ws">WebSocket</option>
                  <option value="h2">HTTP/2</option>
                  <option value="grpc">gRPC</option>
                </select>
              </div>
              <div v-if="addForm.security !== 'none'">
                <label class="form-label">SNI</label>
                <input v-model="addForm.sni" type="text" class="form-input w-full" />
              </div>
              <div>
                <label class="form-label">Flow（XTLS）</label>
                <select v-model="addForm.flow" class="form-input w-full">
                  <option value="">无</option>
                  <option value="xtls-rprx-vision">xtls-rprx-vision</option>
                </select>
              </div>
              <template v-if="addForm.security === 'reality'">
                <div>
                  <label class="form-label">Public Key (pbk)</label>
                  <input v-model="addForm.pbk" type="text" class="form-input w-full font-mono" />
                </div>
                <div>
                  <label class="form-label">Short ID (sid)</label>
                  <input v-model="addForm.sid" type="text" class="form-input w-full font-mono" />
                </div>
              </template>
            </template>

            <!-- shadowsocks -->
            <template v-if="addForm.protocol === 'shadowsocks'">
              <div>
                <label class="form-label">加密方法</label>
                <select v-model="addForm.method" class="form-input w-full">
                  <option v-for="m in SS_METHODS" :key="m" :value="m">{{ m }}</option>
                </select>
              </div>
              <div>
                <label class="form-label">密码</label>
                <input v-model="addForm.password" type="text" class="form-input w-full" />
              </div>
            </template>

            <!-- trojan -->
            <template v-if="addForm.protocol === 'trojan'">
              <div>
                <label class="form-label">密码</label>
                <input v-model="addForm.password" type="text" class="form-input w-full" />
              </div>
              <div>
                <label class="form-label">SNI</label>
                <input v-model="addForm.sni" type="text" class="form-input w-full" :placeholder="addForm.server" />
              </div>
              <div>
                <label class="form-label">传输层</label>
                <select v-model="addForm.transport" class="form-input w-full">
                  <option value="tcp">TCP</option>
                  <option value="ws">WebSocket</option>
                  <option value="grpc">gRPC</option>
                </select>
              </div>
              <div v-if="addForm.transport !== 'tcp'">
                <label class="form-label">Path / Service Name</label>
                <input v-model="addForm.path" type="text" class="form-input w-full" placeholder="/" />
              </div>
            </template>

            <!-- hysteria2 -->
            <template v-if="addForm.protocol === 'hysteria2'">
              <div>
                <label class="form-label">密码</label>
                <input v-model="addForm.password" type="text" class="form-input w-full" />
              </div>
              <div>
                <label class="form-label">SNI</label>
                <input v-model="addForm.sni" type="text" class="form-input w-full" :placeholder="addForm.server" />
              </div>
              <div class="col-span-2 flex items-center gap-2">
                <input type="checkbox" v-model="addForm.insecure" id="hy2-insecure" class="rounded" />
                <label for="hy2-insecure" class="text-sm cursor-pointer">跳过证书验证（insecure）</label>
              </div>
            </template>

            <!-- tuic -->
            <template v-if="addForm.protocol === 'tuic'">
              <div class="col-span-2">
                <label class="form-label">UUID</label>
                <input v-model="addForm.uuid" type="text" class="form-input w-full font-mono" />
              </div>
              <div>
                <label class="form-label">密码</label>
                <input v-model="addForm.password" type="text" class="form-input w-full" />
              </div>
              <div>
                <label class="form-label">SNI</label>
                <input v-model="addForm.sni" type="text" class="form-input w-full" />
              </div>
              <div>
                <label class="form-label">ALPN</label>
                <input v-model="addForm.alpn" type="text" class="form-input w-full" placeholder="h3" />
              </div>
            </template>

            <!-- socks5 / http -->
            <template v-if="addForm.protocol === 'socks5' || addForm.protocol === 'http'">
              <div>
                <label class="form-label">用户名（可选）</label>
                <input v-model="addForm.username" type="text" class="form-input w-full" />
              </div>
              <div>
                <label class="form-label">密码（可选）</label>
                <input v-model="addForm.password" type="text" class="form-input w-full" />
              </div>
            </template>
          </div>

          <p v-if="addError" class="text-sm text-red-500">{{ addError }}</p>
          <div class="flex justify-end gap-2 pt-1">
            <button class="btn-secondary" @click="showAdd = false">取消</button>
            <button class="btn-primary" :disabled="adding" @click="doAdd">{{ adding ? '添加中…' : '添加节点' }}</button>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- ── URI 批量导入 Modal ── -->
    <Teleport to="body">
      <div v-if="showImport" class="modal-overlay" @click.self="closeImport">
        <div class="modal-box space-y-3">
          <h3 class="font-semibold">URI 批量导入</h3>
          <p class="text-xs text-gray-500">每行一个，支持 vmess:// vless:// ss:// trojan:// hy2:// tuic:// socks5:// http://</p>
          <textarea v-model="importText" rows="8" class="form-input font-mono text-xs w-full resize-none"
            placeholder="vmess://...&#10;vless://...&#10;ss://..." />
          <div v-if="importLogs.length" class="max-h-32 overflow-y-auto space-y-0.5 text-xs font-mono bg-gray-50 dark:bg-gray-800 rounded p-2">
            <div v-for="l in importLogs" :key="l.uri" :class="l.ok ? 'text-green-600' : 'text-red-500'">
              {{ l.ok ? '✓' : '✗' }} {{ l.uri.slice(0, 60) }}{{ l.uri.length > 60 ? '…' : '' }}
              <span v-if="!l.ok"> — {{ l.err }}</span>
            </div>
          </div>
          <div class="flex items-center justify-between">
            <span v-if="importDone" class="text-sm text-green-600">完成：成功 {{ importOk }}，失败 {{ importFail }}</span>
            <div class="flex gap-2 ml-auto">
              <button class="btn-secondary" @click="closeImport">{{ importDone ? '关闭' : '取消' }}</button>
              <button v-if="!importDone" class="btn-primary" :disabled="importing" @click="doImport">{{ importing ? '导入中…' : '确认导入' }}</button>
            </div>
          </div>
        </div>
      </div>
    </Teleport>

    <!-- ── URI 查看 Modal ── -->
    <Teleport to="body">
      <div v-if="viewingNode" class="modal-overlay" @click.self="viewingNode = null">
        <div class="modal-box space-y-3">
          <h3 class="font-semibold">节点 URI — {{ viewingNode.name }}</h3>
          <textarea :value="viewingNode.uri" readonly rows="4" class="form-input font-mono text-xs w-full resize-none" />
          <div class="flex justify-end"><button class="btn-secondary" @click="viewingNode = null">关闭</button></div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<script setup>
import { ref, computed, reactive, onMounted } from 'vue'
import { api } from '@/utils/api'
import { useToastStore } from '@/stores/toast'
import { buildUriFromForm } from '@/utils/nodeParser'

const toast = useToastStore()

// ── Data ─────────────────────────────────────────────────────────────────────
const nodes       = ref([])
const search      = ref('')
const filterProto = ref('')
const showEnabled = ref(false)
const selected    = ref(new Set())
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

function toggleAll()       { allSelected.value ? filtered.value.forEach(n => selected.value.delete(n.id)) : filtered.value.forEach(n => selected.value.add(n.id)) }
function toggleSelect(id)  { selected.value.has(id) ? selected.value.delete(id) : selected.value.add(id) }

async function load() {
  try {
    const res = await api.get('/api/nodes')
    nodes.value = Array.isArray(res) ? res : (res.nodes ?? [])
  } catch (e) { toast.error(e.message) }
}

// ── Enable toggle / delete ────────────────────────────────────────────────────
async function toggleEnable(node) {
  try {
    await api.put(`/api/nodes/${node.id}`, { enabled: !node.enabled })
    node.enabled = !node.enabled
  } catch (e) { toast.error(e.message) }
}

async function deleteNode(id) {
  if (!confirm('确认删除该节点？')) return
  try {
    await api.delete(`/api/nodes/${id}`)
    nodes.value  = nodes.value.filter(n => n.id !== id)
    selected.value.delete(id)
    toast.success('已删除')
  } catch (e) { toast.error(e.message) }
}

async function deleteSelected() {
  if (!confirm(`确认删除选中的 ${selected.value.size} 个节点？`)) return
  for (const id of [...selected.value]) {
    try { await api.delete(`/api/nodes/${id}`) } catch {}
  }
  selected.value.clear()
  await load()
  toast.success('批量删除完成')
}

// ── URI view ──────────────────────────────────────────────────────────────────
function viewUri(n) { viewingNode.value = n }

// ── Export ────────────────────────────────────────────────────────────────────
function doExport() {
  const list = filtered.value.map(n => n.uri).filter(Boolean)
  if (!list.length) { toast.error('没有可导出的节点'); return }
  const blob = new Blob([list.join('\n')], { type: 'text/plain' })
  const a    = document.createElement('a')
  a.href     = URL.createObjectURL(blob)
  a.download = 'kitea-nodes.txt'
  a.click()
  URL.revokeObjectURL(a.href)
  toast.success(`已导出 ${list.length} 个节点`)
}

// ── Bulk URI import ───────────────────────────────────────────────────────────
const showImport  = ref(false)
const importText  = ref('')
const importing   = ref(false)
const importDone  = ref(false)
const importOk    = ref(0)
const importFail  = ref(0)
const importLogs  = ref([])

function closeImport() {
  showImport.value = false
  if (importDone.value) { importText.value = ''; importDone.value = false; importLogs.value = []; importOk.value = 0; importFail.value = 0 }
}

async function doImport() {
  const uris = importText.value.split('\n').map(s => s.trim()).filter(Boolean)
  if (!uris.length) return
  importing.value = true
  importLogs.value = []
  importOk.value = importFail.value = 0

  for (const uri of uris) {
    try {
      await api.post('/api/nodes/import', { uri })
      importLogs.value.push({ uri, ok: true })
      importOk.value++
    } catch (e) {
      importLogs.value.push({ uri, ok: false, err: e.message })
      importFail.value++
    }
  }
  importing.value = false
  importDone.value = true
  if (importOk.value > 0) await load()
}

// ── Manual add ────────────────────────────────────────────────────────────────
const PROTOCOLS = [
  { value: 'vmess',       label: 'VMess'       },
  { value: 'vless',       label: 'VLESS'       },
  { value: 'shadowsocks', label: 'Shadowsocks' },
  { value: 'trojan',      label: 'Trojan'      },
  { value: 'hysteria2',   label: 'Hysteria2'   },
  { value: 'tuic',        label: 'TUIC v5'     },
  { value: 'socks5',      label: 'SOCKS5'      },
  { value: 'http',        label: 'HTTP'        },
]

const SS_METHODS = [
  'aes-128-gcm', 'aes-256-gcm', 'chacha20-ietf-poly1305',
  '2022-blake3-aes-128-gcm', '2022-blake3-aes-256-gcm',
  '2022-blake3-chacha20-ietf-poly1305',
]

const showAdd  = ref(false)
const adding   = ref(false)
const addError = ref('')
const addForm  = reactive({
  protocol: 'vmess', name: '', server: '', port: 443,
  uuid: '', cipher: 'aes-128-gcm', tls: 'tls',
  transport: 'tcp', sni: '', path: '/', host: '',
  security: 'tls', flow: '', pbk: '', sid: '',
  method: 'aes-128-gcm', password: '',
  insecure: false, alpn: 'h3',
  username: '',
})

function openAdd() {
  addError.value = ''
  showAdd.value  = true
}

async function doAdd() {
  addError.value = ''
  if (!addForm.server) { addError.value = '服务器地址不能为空'; return }
  if (!addForm.port)   { addError.value = '端口不能为空'; return }
  adding.value = true
  try {
    const uri = buildUriFromForm(addForm)
    await api.post('/api/nodes/import', { uri })
    toast.success('节点已添加')
    showAdd.value = false
    await load()
  } catch (e) {
    addError.value = e.message
  } finally {
    adding.value = false
  }
}

onMounted(load)
</script>

<style scoped>
.protocol-badge { @apply inline-block px-1.5 py-0.5 rounded bg-gray-100 dark:bg-gray-700 text-[10px] font-mono uppercase; }
.modal-overlay  { @apply fixed inset-0 z-50 bg-black/50 flex items-center justify-center p-4; }
.modal-box      { @apply bg-white dark:bg-gray-900 rounded-xl shadow-xl p-6 w-full max-w-lg; }
</style>
