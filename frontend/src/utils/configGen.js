/**
 * configGen.js
 *
 * Generate Shoes-compatible YAML client configuration from parsed node objects.
 *
 * Output format:
 *   - One local mixed (HTTP/SOCKS5) listener on 127.0.0.1:<localPort>
 *   - Rules routing all traffic through enabled node(s) via client_chains
 */

import { parseNodeUri } from './nodeParser'
import yaml from 'js-yaml'

// ── per-protocol client_chain builders ────────────────────────────────────────

function buildVmessChain(node) {
  // Inner protocol
  const vmessProto = {
    type: 'vmess',
    cipher: node.cipher || 'aes-128-gcm',
    user_id: node.uuid,
    udp_enabled: true,
  }

  let proto = vmessProto

  // Handle transport (ws / h2 / grpc)
  if (node.net === 'ws') {
    proto = {
      type: 'websocket',
      host: node.host || node.server,
      path: node.path || '/',
      protocol: vmessProto,
    }
  } else if (node.net === 'h2' || node.net === 'http') {
    proto = {
      type: 'h2',
      host: node.host || node.server,
      path: node.path || '/',
      protocol: vmessProto,
    }
  } else if (node.net === 'grpc') {
    proto = {
      type: 'grpc',
      service_name: node.serviceName || node.path || '',
      protocol: vmessProto,
    }
  }

  // Wrap in TLS if needed
  if (node.tls === 'tls' || node.tls === 'xtls') {
    const tlsProto = {
      type: 'tls',
    }
    if (node.sni) tlsProto.sni_hostname = node.sni
    if (node.alpn) tlsProto.alpn_protocols = node.alpn.split(',').map(s => s.trim())
    if (node.fp)   tlsProto.tls_fingerprint = node.fp
    tlsProto.protocol = proto
    proto = tlsProto
  }

  return {
    address: `${node.server}:${node.port}`,
    protocol: proto,
  }
}

function buildVlessChain(node) {
  const vlessProto = {
    type: 'vless',
    user_id: node.uuid,
    udp_enabled: true,
  }
  if (node.flow === 'xtls-rprx-vision') vlessProto.vision = true

  let proto = vlessProto

  // Transport
  if (node.net === 'ws') {
    proto = { type: 'websocket', host: node.host || node.server, path: node.path || '/', protocol: vlessProto }
  } else if (node.net === 'grpc') {
    proto = { type: 'grpc', service_name: node.serviceName || '', protocol: vlessProto }
  } else if (node.net === 'h2') {
    proto = { type: 'h2', host: node.host || node.server, path: node.path || '/', protocol: vlessProto }
  }

  // Security
  const sec = node.security || ''
  if (sec === 'reality') {
    proto = {
      type:         'reality',
      public_key:   node.pbk || '',
      short_id:     node.sid || '',
      sni_hostname: node.sni || node.server,
      protocol:     proto,
    }
  } else if (sec === 'tls' || sec === 'xtls') {
    const tlsProto = { type: 'tls' }
    if (node.sni)  tlsProto.sni_hostname    = node.sni
    if (node.alpn) tlsProto.alpn_protocols  = node.alpn.split(',').map(s => s.trim())
    if (node.fp)   tlsProto.tls_fingerprint = node.fp
    tlsProto.protocol = proto
    proto = tlsProto
  }

  return { address: `${node.server}:${node.port}`, protocol: proto }
}

function buildShadowsocksChain(node) {
  return {
    address: `${node.server}:${node.port}`,
    protocol: {
      type:     'shadowsocks',
      cipher:   node.method   || 'aes-256-gcm',
      password: node.password || '',
      udp_enabled: true,
    }
  }
}

function buildTrojanChain(node) {
  const trojanProto = {
    type:     'trojan',
    password: node.password || '',
    udp_enabled: true,
  }

  let proto = trojanProto

  if (node.net === 'ws') {
    proto = { type: 'websocket', host: node.host || node.server, path: node.path || '/', protocol: trojanProto }
  } else if (node.net === 'grpc') {
    proto = { type: 'grpc', service_name: node.serviceName || '', protocol: trojanProto }
  }

  const tlsProto = { type: 'tls' }
  if (node.sni)  tlsProto.sni_hostname    = node.sni || node.server
  if (node.alpn) tlsProto.alpn_protocols  = node.alpn.split(',').map(s => s.trim())
  if (node.fp)   tlsProto.tls_fingerprint = node.fp
  tlsProto.protocol = proto

  return { address: `${node.server}:${node.port}`, protocol: tlsProto }
}

function buildHysteria2Chain(node) {
  const entry = {
    address:   `${node.server}:${node.port}`,
    transport: 'quic',
    quic_settings: {},
    protocol: {
      type:     'hysteria2',
      password: node.password || '',
      udp_enabled: true,
    }
  }
  const qs = entry.quic_settings
  if (node.sni)      qs.sni              = node.sni
  if (node.insecure) qs.skip_cert_verify = true
  if (node.alpn)     qs.alpn_protocols   = node.alpn.split(',').map(s => s.trim())
  if (Object.keys(qs).length === 0) delete entry.quic_settings
  return entry
}

function buildTuicChain(node) {
  const entry = {
    address:   `${node.server}:${node.port}`,
    transport: 'quic',
    quic_settings: {},
    protocol: {
      type:     'tuic',
      uuid:     node.uuid     || '',
      password: node.password || '',
      udp_enabled: true,
    }
  }
  const qs = entry.quic_settings
  if (node.sni)      qs.sni              = node.sni
  if (node.insecure) qs.skip_cert_verify = true
  if (node.alpn)     qs.alpn_protocols   = (node.alpn || 'h3').split(',').map(s => s.trim())
  if (Object.keys(qs).length === 0) delete entry.quic_settings
  if (node.zeroRtt)      entry.protocol.zero_rtt_handshake        = true
  return entry
}

function buildSocks5Chain(node) {
  const proto = { type: 'socks5', udp_enabled: true }
  if (node.username) proto.username = node.username
  if (node.password) proto.password = node.password
  return { address: `${node.server}:${node.port}`, protocol: proto }
}

function buildHttpChain(node) {
  const proto = { type: 'http' }
  if (node.username) proto.username = node.username
  if (node.password) proto.password = node.password
  return { address: `${node.server}:${node.port}`, protocol: proto }
}

function buildChain(node) {
  switch (node.protocol) {
    case 'vmess':       return buildVmessChain(node)
    case 'vless':       return buildVlessChain(node)
    case 'shadowsocks': return buildShadowsocksChain(node)
    case 'trojan':      return buildTrojanChain(node)
    case 'hysteria2':   return buildHysteria2Chain(node)
    case 'tuic':        return buildTuicChain(node)
    case 'socks5':      return buildSocks5Chain(node)
    case 'http':        return buildHttpChain(node)
    default:
      throw new Error(`Unsupported protocol: ${node.protocol}`)
  }
}

// ── public API ────────────────────────────────────────────────────────────────

/**
 * Generate Shoes client YAML from a list of enabled node DB records.
 *
 * @param {Array}  nodes       – nodes from the API (each has .uri field)
 * @param {number} localPort   – local mixed proxy port (default 1080)
 * @param {object} options
 * @param {boolean} options.udpEnabled   default true
 * @param {string}  options.dnsServer    optional upstream DNS
 * @returns {string}  YAML string
 */
export function generateShoesConfig(nodes, localPort = 1080, options = {}) {
  if (nodes.length === 0) throw new Error('No enabled nodes')

  const chains = []
  const errors = []

  for (const n of nodes) {
    try {
      const parsed = parseNodeUri(n.uri)
      const chain  = buildChain(parsed)
      chain._comment = n.name  // will be stripped by js-yaml but kept for debugging
      chains.push(chain)
    } catch (e) {
      errors.push(`${n.name}: ${e.message}`)
    }
  }

  if (chains.length === 0) {
    throw new Error('No valid nodes:\n' + errors.join('\n'))
  }

  // Build the local listener doc
  const localListener = {
    address: `127.0.0.1:${localPort}`,
    protocol: { type: 'mixed', udp_enabled: options.udpEnabled !== false },
    rules: [
      {
        masks: '0.0.0.0/0',
        action: 'allow',
        ...(chains.length === 1
          ? { client_chain: clean(chains[0]) }
          : { client_chains: chains.map(c => clean(c)) })
      }
    ]
  }

  // Also add an IPv6 catch-all rule
  const localListenerV6 = {
    ...localListener,
  }
  localListener.rules.push({ masks: '::/0', action: 'allow', ...localListener.rules[0] })
  // Actually, Shoes handles IPv4 and IVv6 in the same masks entry. Let's simplify:
  delete localListener.rules[1]

  // Emit one YAML document
  let yamlStr = '# Generated by KiteA – do not edit manually\n'
  yamlStr += yaml.dump([localListener], { indent: 2, lineWidth: 120, quotingType: '"' })

  if (errors.length > 0) {
    yamlStr += '\n# Skipped nodes (parse errors):\n'
    errors.forEach(e => { yamlStr += `#   ${e}\n` })
  }

  return yamlStr
}

/** Remove undefined / null / _comment fields recursively */
function clean(obj) {
  if (Array.isArray(obj)) return obj.map(clean)
  if (obj && typeof obj === 'object') {
    const out = {}
    for (const [k, v] of Object.entries(obj)) {
      if (k.startsWith('_') || v === undefined || v === null || v === '') continue
      out[k] = clean(v)
    }
    return out
  }
  return obj
}
