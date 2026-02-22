# 🪁 KiteA

> **KiteA** 是一个基于 [Shoes](https://github.com/cfal/shoes) 代理核心的 Web 管理面板，  
> 功能类似 [v2rayA](https://github.com/v2rayA/v2rayA)，全栈使用 **Rust** 编写。

---

## ✨ 功能特性

| 功能 | 说明 |
|------|------|
| 🌐 Web 管理界面 | 响应式单页面应用，支持亮色/暗色/跟随系统 |
| 🔐 账户管理 | JWT 登录鉴权，Admin/User 角色区分，密码修改 |
| 📦 节点管理 | 支持所有 Shoes 协议节点的增删改查与启用/禁用 |
| ▶ 代理控制 | 一键启动/停止 Shoes 子进程，实时状态展示 |
| 📄 配置预览 | 查看自动生成的 Shoes YAML 配置内容 |
| ⚙️ 系统设置 | 配置 Shoes 路径、本地监听端口、日志级别 |
| 🗄 数据持久化 | 使用 SQLite 存储账户、节点、设置信息 |

## 🔧 依赖的 Shoes 协议

Shoes (子模组 `v0.2.7`) 支持以下所有协议：

- **Proxy**: HTTP/HTTPS、SOCKS5、Mixed、VMess AEAD、VLESS、Shadowsocks、Trojan、Snell v3、Hysteria2、TUIC v5、AnyTLS、NaiveProxy、H2MUX
- **Transport**: TLS、ShadowTLS v3、WebSocket、XTLS Reality/Vision、QUIC

---

## 🚀 快速开始

### 前置要求

- Rust 1.75+（推荐使用 MSVC 工具链或安装了 MinGW 的 GNU 工具链）
- [Shoes](https://github.com/cfal/shoes) 二进制文件（已内置为子模组，需单独编译）

### Windows (Scoop + MinGW GNU 工具链)

```powershell
# 1. 确保 MinGW bin 在 PATH 中（解决 dlltool 问题）
$env:PATH = "$env:USERPROFILE\scoop\apps\mingw-winlibs-llvm-ucrt\current\bin;$env:PATH"

# 2. 初始化子模组
git submodule update --init

# 3. 编译 Shoes 二进制
cargo build --release --manifest-path shoes/Cargo.toml
# 或系统安装：cargo install shoes

# 4. 编译 KiteA
cargo build --release

# 5. 运行
./target/release/kite_a
```

### Linux / macOS

```bash
git submodule update --init
cargo install shoes
cargo build --release
./target/release/kite_a
```

---

## ⚙️ 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `KITEA_LISTEN` | `0.0.0.0:7890` | Web 面板监听地址 |
| `KITEA_DB_URL` | `sqlite://kitea.db` | SQLite 数据库路径 |
| `KITEA_JWT_SECRET` | 随机（重启失效） | JWT 签名密钥（建议设置） |
| `KITEA_SHOES_BINARY` | `shoes` | Shoes 可执行文件路径 |
| `KITEA_SHOES_CONFIG` | `shoes_runtime.yaml` | 运行时配置文件路径 |

---

## 📖 使用说明

1. 启动后访问 `http://localhost:7890`
2. **首次运行**自动跳转到注册页，创建管理员账户
3. 登录后进入控制台
4. 在「**节点管理**」中添加代理节点（粘贴 Shoes YAML 配置片段）
5. 在「**系统设置**」中配置 Shoes 二进制路径和本地监听端口
6. 点击「**启动**」按钮启动 Shoes 代理

### 节点 YAML 格式示例（VLESS）

```yaml
- address: your-server.com:443
  protocol:
    type: tls
    tls_targets:
      "your-server.com":
        protocol:
          type: vless
          user_id: your-uuid-here
          udp_enabled: true
```

参考 [Shoes CONFIG.md](https://github.com/cfal/shoes/blob/master/CONFIG.md) 了解完整配置格式。

---

## 🏗 项目结构

```
KiteA/
├── src/
│   └── main.rs          # 全部后端代码（单文件）
├── static/
│   └── index.html       # 前端 SPA（嵌入二进制）
├── shoes/               # Shoes 子模组 (v0.2.7)
├── Cargo.toml
└── .cargo/
    └── config.toml      # 工具链配置说明
```

## 🛠 技术栈

**后端**
- [Actix-web 4](https://actix.rs/) – HTTP 服务器
- [SQLx 0.7](https://github.com/launchbadge/sqlx) + SQLite – 数据持久化
- [jsonwebtoken 9](https://github.com/Keats/jsonwebtoken) – JWT 鉴权
- [bcrypt](https://crates.io/crates/bcrypt) – 密码哈希

**前端**（嵌入式）
- [Tailwind CSS v3 CDN](https://tailwindcss.com/) – 样式
- [Alpine.js v3 CDN](https://alpinejs.dev/) – 响应式交互
- 亮/暗/跟随系统三种主题模式

**代理核心**
- [Shoes v0.2.7](https://github.com/cfal/shoes) – 以子进程方式管理

---

## 📜 许可证

MIT
