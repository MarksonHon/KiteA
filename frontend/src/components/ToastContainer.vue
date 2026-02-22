<template>
  <Teleport to="body">
    <div class="fixed bottom-4 right-4 z-50 flex flex-col-reverse gap-2 pointer-events-none">
      <TransitionGroup name="toast">
        <div
          v-for="t in toasts.list"
          :key="t.id"
          :class="['pointer-events-auto flex items-start gap-3 px-4 py-3 rounded-xl shadow-lg text-sm max-w-xs', colorClass(t.type)]"
        >
          <component :is="iconFor(t.type)" class="w-5 h-5 shrink-0 mt-0.5" />
          <span class="flex-1">{{ t.message }}</span>
          <button class="shrink-0 opacity-60 hover:opacity-100" @click="toasts.remove(t.id)">✕</button>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<script setup>
import { useToastStore } from '@/stores/toast'
import {
  CheckCircleIcon, ExclamationCircleIcon, InformationCircleIcon
} from '@heroicons/vue/24/solid'

const toasts = useToastStore()

function colorClass(type) {
  return {
    success: 'bg-green-600 text-white',
    error:   'bg-red-600 text-white',
    info:    'bg-blue-600 text-white',
    warning: 'bg-yellow-500 text-white',
  }[type] ?? 'bg-gray-700 text-white'
}

function iconFor(type) {
  return { success: CheckCircleIcon, error: ExclamationCircleIcon }[type] ?? InformationCircleIcon
}
</script>

<style scoped>
.toast-enter-active, .toast-leave-active { transition: all .25s ease; }
.toast-enter-from { opacity: 0; transform: translateY(10px); }
.toast-leave-to   { opacity: 0; transform: translateX(40px); }
</style>
