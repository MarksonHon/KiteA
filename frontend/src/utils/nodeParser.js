/**
 * nodeParser.js
 *
 * Parse v2rayN / v2rayA share URI formats into a structured Node object.
 *
 * Supported formats:
 *   vmess://base64(JSON)
 *   vless://uuid@host:port?type=...&security=...&sni=...&flow=...&pbk=...&sid=...#name
 *   ss://base64(method:password)@host:port#name
 *   ss://base64(method:password@host:port)#name  (legacy)
 *   trojan://password@host:port?security=tls&sni=...&type=...#name
 *   hy2://password@host:port?sni=...&insecure=...&obfs=...&obfs-password=...#name
 *   hysteria2://  (alias)
 *   tuic://uuid:password@host:port?sni=...&udp-relay-mode=...#name
 */

function decodeBase64(s) {
  s = s.trim().replace(/-/g, '+').replace(/_/g, '/')
  while (s.length % 4) s += '='
  try {
    return decodeURIComponent(
      atob(s).split('').map(c => '%' + ('00' + c.charCodeAt(0).toString(16)).slice(-2)).join('')
    )
  } catch {
    return atob(s)
  }
}

function urlDecode(s) {
  try { return decodeURIComponent(s) } catch { return s }
}

function parseQuery(search) {
  const p = {}
  for (const [k, v] of new URLSearchParams(search)) p[k] = v
  return p
}

// ── vmess ─────────────────────────────────────────────────────────────────────
function parseVmess(uri) {
  const b64 = uri.slice('vmess://'.length)
  let json
  try { json = JSON.parse(decodeBase64(b64)) }
  catch { throw new Error('Invalid vmess URI: cannot decode JSON') }

  const port = parseInt(json.port) || 443
  return {
    protocol: 'vmess',
    name:     json.ps  || json.add || 'vmess-node',
    server:   json.add || '',
    port,
    // raw fields
    uuid:     json.id  || '',
    cipher:   mapVmessCipher(json.scy || json.cipher || 'auto'),
    // transport
    net:      json.net  || 'tcp',     // tcp / ws / h2 / grpc / quic
    path:     json.path || '/',
    host:     json.host || json.add,
    // tls
    tls:      json.tls  || '',        // '' | 'tls' | 'reality'
    sni:      json.sni  || json.host || json.add,
    fp:       json.fp   || '',
    alpn:     json.alpn || '',
    // aid is deprecated but store it
    alterId:  parseInt(json.aid) || 0,
  }
}

function mapVmessCipher(s) {
  const m = { auto: 'aes-128-gcm', none: 'none',
    'aes-128-gcm': 'aes-128-gcm', 'chacha20-poly1305': 'chacha20-poly1305' }
  return m[s] || 'aes-128-gcm'
}

// ── vless ─────────────────────────────────────────────────────────────────────
function parseVless(uri) {
  // vless://uuid@host:port?params#name
  const withoutScheme = uri.slice('vless://'.length)
  const hashIdx = withoutScheme.indexOf('#')
  const name    = hashIdx >= 0 ? urlDecode(withoutScheme.slice(hashIdx + 1)) : 'vless-node'
  const main    = withoutScheme.slice(0, hashIdx >= 0 ? hashIdx : undefined)
  const qIdx    = main.indexOf('?')
  const q       = qIdx >= 0 ? parseQuery(main.slice(qIdx + 1)) : {}
  const userhost = main.slice(0, qIdx >= 0 ? qIdx : undefined)
  const atIdx   = userhost.indexOf('@')
  const uuid    = atIdx >= 0 ? userhost.slice(0, atIdx) : ''
  const hostport = userhost.slice(atIdx + 1)
  const colonIdx = hostport.lastIndexOf(':')
  const server  = hostport.slice(0, colonIdx)
  const port    = parseInt(hostport.slice(colonIdx + 1)) || 443

  return {
    protocol: 'vless',
    name, server, port, uuid,
    // transport
    net:      q.type    || q.net || 'tcp',
    path:     q.path    || '/',
    host:     q.host    || server,
    serviceName: q.serviceName || '',  // for grpc
    // security
    security: q.security || '',        // '' | 'tls' | 'reality' | 'none'
    sni:      q.sni     || q.serverName || server,
    fp:       q.fp      || '',
    alpn:     q.alpn    || '',
    flow:     q.flow    || '',          // xtls-rprx-vision
    // reality
    pbk:      q.pbk     || '',          // public key
    sid:      q.sid     || '',          // short id
  }
}

// ── shadowsocks ───────────────────────────────────────────────────────────────
function parseShadowsocks(uri) {
  // ss://base64(method:password)@host:port#name    (SIP002)
  // ss://base64(method:password@host:port)#name    (legacy)
  const withoutScheme = uri.slice('ss://'.length)
  const hashIdx = withoutScheme.indexOf('#')
  const name    = hashIdx >= 0 ? urlDecode(withoutScheme.slice(hashIdx + 1)) : 'ss-node'
  const main    = withoutScheme.slice(0, hashIdx >= 0 ? hashIdx : undefined)

  let method, password, server, port

  if (main.includes('@')) {
    // SIP002: base64(method:password)@host:port
    const atIdx   = main.lastIndexOf('@')
    const b64part = main.slice(0, atIdx)
    const hostport = main.slice(atIdx + 1)

    let decoded
    try { decoded = decodeBase64(b64part) } catch { decoded = b64part }

    const colonIdx = decoded.indexOf(':')
    method   = decoded.slice(0, colonIdx)
    password = decoded.slice(colonIdx + 1)

    const hpColon = hostport.lastIndexOf(':')
    server = hostport.slice(0, hpColon)
    port   = parseInt(hostport.slice(hpColon + 1)) || 8388
  } else {
    // Legacy: base64(method:password@host:port)
    let decoded
    try { decoded = decodeBase64(main) } catch { decoded = main }
    // decoded = "method:password@host:port"
    const atIdx   = decoded.lastIndexOf('@')
    const mp      = decoded.slice(0, atIdx)
    const hostport = decoded.slice(atIdx + 1)
    const c1 = mp.indexOf(':')
    method   = mp.slice(0, c1)
    password = mp.slice(c1 + 1)
    const hpColon = hostport.lastIndexOf(':')
    server = hostport.slice(0, hpColon)
    port   = parseInt(hostport.slice(hpColon + 1)) || 8388
  }

  return { protocol: 'shadowsocks', name, server, port, method, password }
}

// ── trojan ────────────────────────────────────────────────────────────────────
function parseTrojan(uri) {
  const withoutScheme = uri.slice('trojan://'.length)
  const hashIdx = withoutScheme.indexOf('#')
  const name    = hashIdx >= 0 ? urlDecode(withoutScheme.slice(hashIdx + 1)) : 'trojan-node'
  const main    = withoutScheme.slice(0, hashIdx >= 0 ? hashIdx : undefined)
  const qIdx    = main.indexOf('?')
  const q       = qIdx >= 0 ? parseQuery(main.slice(qIdx + 1)) : {}
  const userhost = main.slice(0, qIdx >= 0 ? qIdx : undefined)
  const atIdx   = userhost.lastIndexOf('@')
  const password = atIdx >= 0 ? urlDecode(userhost.slice(0, atIdx)) : ''
  const hostport = userhost.slice(atIdx + 1)
  const colonIdx = hostport.lastIndexOf(':')
  const server  = hostport.slice(0, colonIdx)
  const port    = parseInt(hostport.slice(colonIdx + 1)) || 443

  return {
    protocol: 'trojan', name, server, port, password,
    net:      q.type || q.net || 'tcp',
    path:     q.path || '/',
    host:     q.host || server,
    security: q.security || 'tls',
    sni:      q.sni  || q.serverName || server,
    fp:       q.fp   || '',
    alpn:     q.alpn || '',
  }
}

// ── hysteria2 ─────────────────────────────────────────────────────────────────
function parseHysteria2(uri) {
  const scheme = uri.startsWith('hysteria2://') ? 'hysteria2://' : 'hy2://'
  const withoutScheme = uri.slice(scheme.length)
  const hashIdx = withoutScheme.indexOf('#')
  const name    = hashIdx >= 0 ? urlDecode(withoutScheme.slice(hashIdx + 1)) : 'hy2-node'
  const main    = withoutScheme.slice(0, hashIdx >= 0 ? hashIdx : undefined)
  const qIdx    = main.indexOf('?')
  const q       = qIdx >= 0 ? parseQuery(main.slice(qIdx + 1)) : {}
  const userhost = main.slice(0, qIdx >= 0 ? qIdx : undefined)
  const atIdx   = userhost.lastIndexOf('@')
  const password = atIdx >= 0 ? urlDecode(userhost.slice(0, atIdx)) : ''
  const hostport = userhost.slice(atIdx + 1)
  const colonIdx = hostport.lastIndexOf(':')
  const server  = hostport.slice(0, colonIdx)
  const port    = parseInt(hostport.slice(colonIdx + 1)) || 443

  return {
    protocol: 'hysteria2', name, server, port, password,
    sni:      q.sni        || server,
    insecure: q.insecure   === '1',
    obfs:     q.obfs       || '',
    obfsPassword: q['obfs-password'] || '',
    pinSHA256: q.pinSHA256 || '',
  }
}

// ── tuic ──────────────────────────────────────────────────────────────────────
function parseTuic(uri) {
  const withoutScheme = uri.slice('tuic://'.length)
  const hashIdx = withoutScheme.indexOf('#')
  const name    = hashIdx >= 0 ? urlDecode(withoutScheme.slice(hashIdx + 1)) : 'tuic-node'
  const main    = withoutScheme.slice(0, hashIdx >= 0 ? hashIdx : undefined)
  const qIdx    = main.indexOf('?')
  const q       = qIdx >= 0 ? parseQuery(main.slice(qIdx + 1)) : {}
  const userhost = main.slice(0, qIdx >= 0 ? qIdx : undefined)
  const atIdx   = userhost.lastIndexOf('@')
  const userInfo = atIdx >= 0 ? userhost.slice(0, atIdx) : ''
  const hostport = userhost.slice(atIdx + 1)
  const colonInUser = userInfo.indexOf(':')
  const uuid     = colonInUser >= 0 ? userInfo.slice(0, colonInUser) : userInfo
  const password = colonInUser >= 0 ? userInfo.slice(colonInUser + 1) : ''
  const colonIdx = hostport.lastIndexOf(':')
  const server  = hostport.slice(0, colonIdx)
  const port    = parseInt(hostport.slice(colonIdx + 1)) || 443

  return {
    protocol: 'tuic', name, server, port, uuid, password,
    sni:      q.sni           || server,
    alpn:     q.alpn          || 'h3',
    udpRelayMode: q['udp-relay-mode'] || 'native',
    congestion:   q['congestion-control'] || 'cubic',
    zeroRtt:  q['zero-rtt-handshake'] === '1',
    insecure: q.insecure === '1',
  }
}

// ── socks5 ────────────────────────────────────────────────────────────────────
function parseSocks5(uri) {
  const withoutScheme = uri.slice('socks5://'.length)
  const hashIdx = withoutScheme.indexOf('#')
  const name    = hashIdx >= 0 ? urlDecode(withoutScheme.slice(hashIdx + 1)) : 'socks5-node'
  const main    = (hashIdx >= 0 ? withoutScheme.slice(0, hashIdx) : withoutScheme).split('?')[0]
  let username = '', password = '', hostport = main
  if (main.includes('@')) {
    const atIdx = main.lastIndexOf('@')
    const creds = main.slice(0, atIdx)
    hostport = main.slice(atIdx + 1)
    const ci = creds.indexOf(':')
    username = ci >= 0 ? creds.slice(0, ci) : creds
    password = ci >= 0 ? creds.slice(ci + 1) : ''
  }
  const ci = hostport.lastIndexOf(':')
  return { protocol: 'socks5', name, server: hostport.slice(0, ci), port: parseInt(hostport.slice(ci + 1)) || 1080, username, password }
}

// ── http proxy ────────────────────────────────────────────────────────────────
function parseHttpProxy(uri) {
  const withoutScheme = uri.slice('http://'.length)
  const hashIdx = withoutScheme.indexOf('#')
  const name    = hashIdx >= 0 ? urlDecode(withoutScheme.slice(hashIdx + 1)) : 'http-node'
  const main    = (hashIdx >= 0 ? withoutScheme.slice(0, hashIdx) : withoutScheme).split('?')[0]
  let username = '', password = '', hostport = main
  if (main.includes('@')) {
    const atIdx = main.lastIndexOf('@')
    const creds = main.slice(0, atIdx)
    hostport = main.slice(atIdx + 1)
    const ci = creds.indexOf(':')
    username = ci >= 0 ? creds.slice(0, ci) : creds
    password = ci >= 0 ? creds.slice(ci + 1) : ''
  }
  const ci = hostport.lastIndexOf(':')
  return { protocol: 'http', name, server: hostport.slice(0, ci), port: parseInt(hostport.slice(ci + 1)) || 8080, username, password }
}

// ── public API ────────────────────────────────────────────────────────────────
/**
 * Parse a single share URI and return a structured node object.
 * @param {string} uri
 * @returns {object} parsed node
 */
export function parseNodeUri(uri) {
  uri = uri.trim()
  if (uri.startsWith('vmess://'))                       return parseVmess(uri)
  if (uri.startsWith('vless://'))                       return parseVless(uri)
  if (uri.startsWith('ss://'))                          return parseShadowsocks(uri)
  if (uri.startsWith('trojan://'))                      return parseTrojan(uri)
  if (uri.startsWith('hy2://') || uri.startsWith('hysteria2://')) return parseHysteria2(uri)
  if (uri.startsWith('tuic://'))                        return parseTuic(uri)
  if (uri.startsWith('socks5://'))                      return parseSocks5(uri)
  if (uri.startsWith('http://'))                        return parseHttpProxy(uri)
  throw new Error(`Unsupported URI scheme: ${uri.slice(0, 20)}...`)
}

/**
 * Build a share URI from a manual-add form object.
 * Returns a URI string compatible with parseNodeUri().
 */
export function buildUriFromForm(form) {
  const name = encodeURIComponent(form.name || `${form.protocol}-${form.server}`)
  switch (form.protocol) {
    case 'vmess': {
      const obj = {
        v: '2', ps: form.name || form.server, add: form.server,
        port: String(form.port), id: form.uuid, aid: '0',
        scy: form.cipher || 'aes-128-gcm',
        net: form.transport || 'tcp', type: 'none',
        host: form.host || '', path: form.path || '/',
        tls: form.tls || '',  sni: form.sni || '',
      }
      return 'vmess://' + btoa(encodeURIComponent(JSON.stringify(obj)).replace(/%([0-9A-F]{2})/g, (_, p) => String.fromCharCode('0x' + p)))
    }
    case 'vless': {
      const p = new URLSearchParams()
      p.set('type', form.transport || 'tcp')
      p.set('security', form.security || 'tls')
      if (form.sni)  p.set('sni',  form.sni)
      if (form.flow) p.set('flow', form.flow)
      if (form.pbk)  p.set('pbk',  form.pbk)
      if (form.sid)  p.set('sid',  form.sid)
      if (form.path) p.set('path', form.path)
      if (form.host) p.set('host', form.host)
      return `vless://${form.uuid}@${form.server}:${form.port}?${p}#${name}`
    }
    case 'shadowsocks': {
      const userinfo = btoa(`${form.method}:${form.password}`)
      return `ss://${userinfo}@${form.server}:${form.port}#${name}`
    }
    case 'trojan': {
      const p = new URLSearchParams()
      p.set('security', 'tls')
      if (form.sni) p.set('sni', form.sni)
      if (form.transport && form.transport !== 'tcp') {
        p.set('type', form.transport)
        if (form.path) p.set('path', form.path)
        if (form.host) p.set('host', form.host)
      }
      return `trojan://${encodeURIComponent(form.password)}@${form.server}:${form.port}?${p}#${name}`
    }
    case 'hysteria2': {
      const p = new URLSearchParams()
      if (form.sni)      p.set('sni', form.sni)
      if (form.insecure) p.set('insecure', '1')
      if (form.obfs)     { p.set('obfs', form.obfs); if (form.obfsPassword) p.set('obfs-password', form.obfsPassword) }
      return `hy2://${encodeURIComponent(form.password)}@${form.server}:${form.port}?${p}#${name}`
    }
    case 'tuic': {
      const p = new URLSearchParams()
      if (form.sni)  p.set('sni',  form.sni)
      if (form.alpn) p.set('alpn', form.alpn)
      if (form.insecure) p.set('insecure', '1')
      return `tuic://${form.uuid}:${encodeURIComponent(form.password)}@${form.server}:${form.port}?${p}#${name}`
    }
    case 'socks5': {
      if (form.username) return `socks5://${encodeURIComponent(form.username)}:${encodeURIComponent(form.password || '')}@${form.server}:${form.port}#${name}`
      return `socks5://${form.server}:${form.port}#${name}`
    }
    case 'http': {
      if (form.username) return `http://${encodeURIComponent(form.username)}:${encodeURIComponent(form.password || '')}@${form.server}:${form.port}#${name}`
      return `http://${form.server}:${form.port}#${name}`
    }
    default:
      throw new Error('Unsupported protocol: ' + form.protocol)
  }
}

/**
 * Parse a subscription content (plain text or base64-encoded list of URIs).
 * Returns an array of URI strings.
 * @param {string} content  Raw subscription content from URL
 */
export function parseSubscriptionContent(content) {
  content = content.trim()
  // Try base64 decode
  let decoded = content
  try {
    const d = decodeBase64(content)
    // If decoded content looks like URIs, use it
    if (d.includes('://')) decoded = d
  } catch { /* not base64 */ }

  return decoded
    .split(/\r?\n/)
    .map(l => l.trim())
    .filter(l => l.includes('://') && !l.startsWith('#'))
}
