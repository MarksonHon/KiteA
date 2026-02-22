import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const routes = [
  { path: '/setup',   name: 'Setup',   component: () => import('@/views/SetupView.vue'),   meta: { public: true } },
  { path: '/login',   name: 'Login',   component: () => import('@/views/LoginView.vue'),   meta: { public: true } },
  {
    path: '/',
    component: () => import('@/components/Layout.vue'),
    children: [
      { path: '',             redirect: '/dashboard' },
      { path: 'dashboard',    name: 'Dashboard',   component: () => import('@/views/DashboardView.vue') },
      { path: 'nodes',        name: 'Nodes',       component: () => import('@/views/NodesView.vue') },
      { path: 'subscriptions',name: 'Subscriptions',component: () => import('@/views/SubscriptionsView.vue') },
      { path: 'settings',     name: 'Settings',    component: () => import('@/views/SettingsView.vue') },
      { path: 'users',        name: 'Users',       component: () => import('@/views/UsersView.vue') },
      { path: 'account',      name: 'Account',     component: () => import('@/views/AccountView.vue') },
    ]
  },
  { path: '/:pathMatch(.*)*', redirect: '/dashboard' }
]

const router = createRouter({
  history: createWebHistory(),
  routes
})

// 缓存 setup 状态，避免每次导航都请求后端
let setupNeededCache = null
async function isSetupNeeded() {
  if (setupNeededCache !== null) return setupNeededCache
  try {
    const res  = await fetch('/api/setup/needed')
    const json = await res.json()
    setupNeededCache = !!json.data?.needed
  } catch {
    setupNeededCache = false
  }
  return setupNeededCache
}

// 初始化完成后将缓存失效（setup 完成后下次导航会重新检测）
export function invalidateSetupCache() {
  setupNeededCache = null
}

router.beforeEach(async (to) => {
  const auth = useAuthStore()

  // 访问 /setup：若已有用户则跳到登录页
  if (to.name === 'Setup') {
    const needed = await isSetupNeeded()
    return needed ? true : '/login'
  }

  // 访问 /login：若尚无用户则跳到初始化页
  if (to.name === 'Login') {
    const needed = await isSetupNeeded()
    return needed ? '/setup' : true
  }

  // 受保护路由：未登录时先判断是否需要初始化
  if (!auth.token) {
    const needed = await isSetupNeeded()
    return needed ? '/setup' : '/login'
  }

  return true
})

export default router
