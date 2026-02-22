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

    <!-- TUN inbound settings -->
    <div class="card p-6 space-y-4">
      <h3 class="font-medium">TUN 透明代理（hev-socks5-tunnel）</h3>
      <p class="text-xs text-gray-500">
        启用后将在代理启动时同步启动 hev-socks5-tunnel，创建 TUN 虚拟网卡并将系统全局流量路由至 Shoes SOCKS5 入口。
        需要 <a href="https://github.com/heiher/hev-socks5-tunnel" target="_blank" class="underline">hev-socks5-tunnel</a>
        已安装，并通过 <code>setcap cap_net_admin+ep &lt;路径&gt;</code> 授权或以 root 权限运行 KiteA。
      </p>
      <div class="flex items-center gap-3">
        <label class="form-label mb-0 cursor-pointer" for="tun-enabled">启用 TUN 模式</label>
        <input id="tun-enabled" type="checkbox" v-model="tunEnabled" class="rounded w-4 h-4" />
      </div>
      <div>
        <label class="form-label">hev-socks5-tunnel 可执行文件路径</label>
        <input v-model="form.tun_binary" type="text" class="form-input w-full" placeholder="hev-socks5-tunnel" />
      </div>
      <div>
        <label class="form-label">TUN 设备名称</label>
        <input v-model="form.tun_name" type="text" class="form-input w-40" placeholder="tun0" />
      </div>
      <div>
        <label class="form-label">TUN IPv4 地址</label>
        <input v-model="form.tun_ipv4" type="text" class="form-input w-40" placeholder="198.18.0.1" />
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
import { ref, computed, onMounted } from 'vue'
import { api } from '@/utils/api'
import { useToastStore } from '@/stores/toast'

const toast = useToastStore()
const saving = ref(false)
const saved  = ref(false)
const form   = ref({ local_port: 1080, shoes_bin: '', dns_server: '', tun_enabled: 'false', tun_binary: 'hev-socks5-tunnel', tun_name: 'tun0', tun_ipv4: '198.18.0.1' })

// Boolean proxy for the tun_enabled string setting
const tunEnabled = computed({
  get: () => form.value.tun_enabled === 'true',
  set: (v) => { form.value.tun_enabled = v ? 'true' : 'false' },
})

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
