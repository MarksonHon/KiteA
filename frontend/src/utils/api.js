import { useAuthStore } from '@/stores/auth'
import router from '@/router'

const BASE = ''  // same origin when served by backend; proxied in dev

async function request(method, path, body) {
  const auth = useAuthStore()
  const headers = { 'Content-Type': 'application/json' }
  if (auth.token) headers['Authorization'] = `Bearer ${auth.token}`

  const res = await fetch(BASE + path, {
    method,
    headers,
    body: body !== undefined ? JSON.stringify(body) : undefined
  })

  const json = await res.json().catch(() => null)

  if (res.status === 401) {
    auth.clearAuth()
    router.push('/login')
    throw new Error('Unauthorised')
  }

  if (!json?.ok) {
    throw new Error(json?.msg || `HTTP ${res.status}`)
  }

  // 返回 data 字段，使调用方可直接 res.token / res.nodes 等
  return json.data !== undefined ? json.data : json
}

export const api = {
  get:    (path)        => request('GET',    path),
  post:   (path, body)  => request('POST',   path, body),
  put:    (path, body)  => request('PUT',    path, body),
  delete: (path)        => request('DELETE', path),
}
