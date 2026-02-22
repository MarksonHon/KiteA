<template>
  <div class="min-h-screen flex bg-gray-50 dark:bg-gray-950 text-gray-900 dark:text-gray-100">

    <!-- ── Mobile overlay ── -->
    <div
      v-if="sidebarOpen"
      class="fixed inset-0 z-20 bg-black/50 lg:hidden"
      @click="sidebarOpen = false"
    />

    <!-- ── Sidebar ── -->
    <aside
      :class="[
        'fixed inset-y-0 left-0 z-30 w-56 flex flex-col bg-white dark:bg-gray-900 border-r border-gray-200 dark:border-gray-800 transition-transform duration-200',
        sidebarOpen ? 'translate-x-0' : '-translate-x-full lg:translate-x-0'
      ]"
    >
      <!-- Logo -->
      <div class="h-14 flex items-center px-4 border-b border-gray-200 dark:border-gray-800 shrink-0">
        <span class="text-lg font-bold tracking-tight text-primary-600 dark:text-primary-400">KiteA</span>
        <span class="ml-2 text-xs text-gray-400">v2rayA-like</span>
      </div>

      <!-- Nav -->
      <nav class="flex-1 overflow-y-auto py-3 space-y-0.5 px-2">
        <RouterLink
          v-for="item in navItems"
          :key="item.to"
          :to="item.to"
          class="nav-link"
          active-class="nav-link-active"
          @click="sidebarOpen = false"
        >
          <component :is="item.icon" class="w-4 h-4 shrink-0" />
          <span>{{ item.label }}</span>
        </RouterLink>
      </nav>

      <!-- Bottom -->
      <div class="p-2 border-t border-gray-200 dark:border-gray-800 space-y-0.5">
        <!-- Theme toggle -->
        <button class="nav-link w-full" @click="cycleTheme">
          <SunIcon  v-if="theme.current === 'light'" class="w-4 h-4 shrink-0" />
          <MoonIcon v-else-if="theme.current === 'dark'" class="w-4 h-4 shrink-0" />
          <ComputerDesktopIcon v-else class="w-4 h-4 shrink-0" />
          <span>{{ themeLabel }}</span>
        </button>
        <!-- Logout -->
        <button class="nav-link w-full text-red-500 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20" @click="handleLogout">
          <ArrowRightOnRectangleIcon class="w-4 h-4 shrink-0" />
          <span>退出登录</span>
        </button>
      </div>
    </aside>

    <!-- ── Main ── -->
    <div class="flex-1 flex flex-col lg:ml-56 min-w-0">
      <!-- Topbar -->
      <header class="h-14 shrink-0 flex items-center gap-3 px-4 border-b border-gray-200 dark:border-gray-800 bg-white/80 dark:bg-gray-900/80 backdrop-blur sticky top-0 z-10">
        <button class="lg:hidden p-1 rounded hover:bg-gray-100 dark:hover:bg-gray-800" @click="sidebarOpen = !sidebarOpen">
          <Bars3Icon class="w-5 h-5" />
        </button>
        <span class="font-medium text-sm">{{ currentTitle }}</span>
        <div class="ml-auto flex items-center gap-2 text-xs text-gray-500 dark:text-gray-400">
          <span>{{ auth.username }}</span>
          <span v-if="auth.role === 'admin'" class="badge-admin">管理员</span>
        </div>
      </header>

      <!-- Page content -->
      <main class="flex-1 p-4 overflow-auto">
        <RouterView />
      </main>
    </div>

    <!-- ── Toast ── -->
    <ToastContainer />
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '@/stores/auth'
import { useThemeStore } from '@/stores/theme'
import ToastContainer from '@/components/ToastContainer.vue'

import {
  HomeIcon, ServerIcon, RssIcon, CogIcon, UsersIcon, UserCircleIcon,
  SunIcon, MoonIcon, ComputerDesktopIcon, ArrowRightOnRectangleIcon, Bars3Icon
} from '@heroicons/vue/24/outline'

const router   = useRouter()
const auth     = useAuthStore()
const theme    = useThemeStore()
const sidebarOpen = ref(false)

const navItems = computed(() => {
  const base = [
    { to: '/dashboard',     label: '仪表盘',   icon: HomeIcon },
    { to: '/nodes',         label: '节点管理', icon: ServerIcon },
    { to: '/subscriptions', label: '订阅管理', icon: RssIcon },
    { to: '/settings',      label: '系统设置', icon: CogIcon },
    { to: '/account',       label: '我的账户', icon: UserCircleIcon },
  ]
  if (auth.role === 'admin') base.splice(4, 0, { to: '/users', label: '用户管理', icon: UsersIcon })
  return base
})

const currentTitle = computed(() => {
  const found = navItems.value.find(i => router.currentRoute.value.path.startsWith(i.to))
  return found?.label ?? 'KiteA'
})

const themeLabel = computed(() => ({ system: '跟随系统', light: '浅色', dark: '深色' })[theme.current])

function cycleTheme() {
  const order = ['system', 'light', 'dark']
  const idx   = order.indexOf(theme.current)
  theme.set(order[(idx + 1) % 3])
}

function handleLogout() {
  auth.logout()
  router.push('/login')
}
</script>

<style scoped>
.nav-link {
  @apply flex items-center gap-2.5 px-3 py-2 rounded-md text-sm text-gray-700 dark:text-gray-300
         hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors cursor-pointer;
}
.nav-link-active {
  @apply bg-primary-50 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300 font-medium;
}
.badge-admin {
  @apply px-1.5 py-0.5 rounded text-xs bg-primary-100 dark:bg-primary-900/40 text-primary-600 dark:text-primary-400;
}
</style>
