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

router.beforeEach(async (to) => {
  if (to.meta.public) return true
  const auth = useAuthStore()
  if (!auth.token) return '/login'
  return true
})

export default router
