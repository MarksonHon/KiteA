<template>
  <div class="space-y-4 max-w-md">
    <h2 class="text-xl font-semibold">我的账户</h2>

    <div class="card p-6 space-y-4">
      <div class="flex items-center gap-4">
        <div class="w-12 h-12 rounded-full bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center text-xl font-bold text-primary-600 dark:text-primary-400">
          {{ auth.username?.[0]?.toUpperCase() }}
        </div>
        <div>
          <div class="font-semibold">{{ auth.username }}</div>
          <div class="text-xs text-gray-500">{{ auth.role }}</div>
        </div>
      </div>

      <hr class="border-gray-200 dark:border-gray-700" />

      <h3 class="font-medium text-sm">修改密码</h3>
      <div class="space-y-3">
        <div>
          <label class="form-label">原密码</label>
          <input v-model="form.old_password" type="password" class="form-input w-full" autocomplete="current-password" />
        </div>
        <div>
          <label class="form-label">新密码</label>
          <input v-model="form.new_password" type="password" class="form-input w-full" autocomplete="new-password" />
        </div>
        <div>
          <label class="form-label">确认新密码</label>
          <input v-model="form.confirm" type="password" class="form-input w-full" autocomplete="new-password" />
        </div>
        <button class="btn-primary" :disabled="saving" @click="changePassword">
          {{ saving ? '保存中…' : '修改密码' }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue'
import { api } from '@/utils/api'
import { useToastStore } from '@/stores/toast'
import { useAuthStore } from '@/stores/auth'

const toast  = useToastStore()
const auth   = useAuthStore()
const saving = ref(false)
const form   = ref({ old_password: '', new_password: '', confirm: '' })

async function changePassword() {
  if (form.value.new_password !== form.value.confirm) { toast.error('两次密码不一致'); return }
  if (!form.value.new_password) { toast.error('密码不能为空'); return }
  saving.value = true
  try {
    await api.post('/api/auth/change-password', {
      old_password: form.value.old_password,
      new_password: form.value.new_password,
    })
    toast.success('密码已修改')
    form.value = { old_password: '', new_password: '', confirm: '' }
  } catch (e) { toast.error(e.message) }
  finally { saving.value = false }
}
</script>
