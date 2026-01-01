# react-native-nitro-dns 🚀

**react-native-nitro-dns** 是一个基于 [Nitro Modules](https://nitro.margelo.com/) 和 Rust 构建的高性能 React Native DNS 模块。它提供了一套与 Node.js `dns` 模块高度兼容的 API，并针对移动端扩展了现代隐私协议支持（DoH, DoT, DoQ）以及全局网络流量拦截功能。

[![license](https://img.shields.io/badge/license-ISC-blue.svg)](https://github.com/iwater/rn-http-server/blob/main/LICENSE)
[![platform](https://img.shields.io/badge/platform-ios%20%7C%20android-lightgrey.svg)]()
[![compatibility](https://img.shields.io/badge/Node.js-100%25%20dns-green.svg)]()

---

## ✨ 特性

- 🚀 **极致性能**：核心逻辑由 Rust 实现，通过 Nitro Modules (JSI/C++) 实现零拷贝调用，性能远超传统的 Bridge。
- 📦 **Node.js 高度兼容**：支持 `lookup`, `resolve*`, `reverse`, `lookupService` 以及 `dns.promises` API。
- 🛡️ **现代隐私协议**：
  - **DoH** (DNS over HTTPS) - 支持自定义路径。
  - **DoT** (DNS over TLS)。
  - **DoQ** (DNS over QUIC)。
- 🔌 **全局网络拦截**：一键拦截 App 内所有原生请求（`fetch`, `XMLHttpRequest` 等）并应用自定义解析规则。
- 🌐 **跨平台行为一致**：Android (通过 OkHttp Hook 注入) 和 iOS (基于 NSURLProtocol) 实现，逻辑完全同步。
- 🧩 **进阶配置**：支持 SNI 覆盖（Bootstrap IP）、超时管理、重试次数及 IPv4/IPv6 优先级排序。
- ⚡ **增强缓存**：Rust 原生层级缓存，支持 **Stale-While-Revalidate (SWR)** 和 **Stale-If-Error (SIE)** 策略。

---

## 📥 安装

```bash
# 使用 npm
npm install react-native-nitro-dns

# 使用 yarn
yarn add react-native-nitro-dns
```

### iOS 配置

iOS 端基于 `NSURLProtocol` 实现，由 Nitro 自动链接集成，通常不需要额外手动配置 `AppDelegate`。

### Android 配置

为了启用 Android 端的全局 OkHttp 拦截，你需要在 `MainApplication.kt` (或 `MainApplication.java`) 中注入 Nitro 提供的工厂类：

```kotlin
// MainApplication.kt (针对 React Native 0.73+)
import com.nitrodns.NitroOkHttpClientFactory
import com.facebook.react.modules.network.OkHttpClientProvider

override fun onCreate() {
    super.onCreate()
    // 注入 Nitro DNS 工厂以支持全局 fetch/XHR 拦截
    OkHttpClientProvider.setOkHttpClientFactory(NitroOkHttpClientFactory())
}
```

---

## 🚀 快速上手

### 1. 域名解析 (Node.js 风格)

```typescript
import dns from 'react-native-nitro-dns';

// 使用 Promise APIs
const { address, family } = await dns.promises.lookup('google.com');

// 使用 resolve4 获取所有 IP
const addresses = await dns.promises.resolve4('example.com');
```

### 2. 配置加密 DNS

```typescript
// 配置 DoH (带有自定义路径)
dns.setServers(['https://dns.alidns.com/dns-query']);

// 配置 DoT
dns.setServers(['tls://dns.google']);

// 配置多种协议混合 (会自动负载均衡或按顺序尝试)
dns.setServers(['quic://dns.nextdns.io', '8.8.8.8']);
```

### 3. 全局流量拦截 (原生层)

一旦开启，App 内所有的网络请求（如 `fetch('https://...')`）都将使用 Nitro DNS 的解析结果，而不是系统的 DNS 缓存。

```typescript
// 开启拦截
dns.setNativeInterceptionEnabled(true);

// 此 fetch 请求将受到 Nitro DNS 控制 (包括 DoH/DoT 等)
const res = await fetch('https://my-secure-api.com');
```

### 4. 高性能缓存控制

通过灵活的缓存策略优化解析速度和可靠性。

```typescript
import dns, { CachePolicy } from 'react-native-nitro-dns';

// 设置全局缓存条目上限
dns.setCacheSize(1000);

// 设置全局缓存策略为 SWR (过期重新验证)
// 如果存在过期数据则立即返回，并在后台异步刷新结果
dns.setCachePolicy(CachePolicy.StaleWhileRevalidate, 86400); // 1天过期宽限期
```

---

## 📖 API 详细参考

### `dns.promises` 高级 API

| 方法 | 参数说明 | 默认返回值 | 进阶用法 |
| :--- | :--- | :--- | :--- |
| `lookup` | `hostname, options?` | `{address, family}` | `options` 支持 `all`, `verbatim`, `family` |
| `resolve4` | `hostname, options?` | `string[]` | 传 `{ttl: true}` 可获取 `[{address, ttl}]` |
| `resolve6` | `hostname, options?` | `string[]` | 传 `{ttl: true}` 可获取 `[{address, ttl}]` |
| `resolveMx` | `hostname` | `{exchange, priority}[]` | 解析邮件交换记录 |
| `resolveTxt` | `hostname` | `string[][]` | 解析文本记录 |
| `resolveTlsa` | `hostname` | `TLSA[]` | 解析 DANE 安全证书指纹 |
| `lookupService` | `address, port` | `{hostname, service}` | 根据 IP 端口查主机名 |
| `clearCache` | - | `void` | 清空所有 DNS 缓存 |
| `setCacheSize` | `size: number` | `void` | 设置全局缓存容量 (默认 32) |
| `setCachePolicy`| `policy, staleTtl` | `void` | 设置全局缓存策略 (支持 SWR/SIE) |

### 常量支持

模块导出了以下 Node.js 标准常量，用于 `lookup` 的 `hints`：
- `ADDRCONFIG`
- `V4MAPPED`
- `ALL`

---

### 4. 域名分流路由 (dnsmasq 风格)

支持针对特定后缀的域名使用专门的解析服务器，其余域名使用默认服务器。

**格式：`/domain/server_url`**

```typescript
dns.setServers([
  '/google.com/8.8.8.8',      // google.com 及其子域使用 8.8.8.8
  '/corp.local/10.0.0.1',     // 内部域名使用公司 DNS
  '1.1.1.1'                   // 其他所有域名使用 1.1.1.1 (默认服务器)
]);
```

### 5. 指代系统 DNS (`system`)

在分流规则中，可以使用 `system` 关键字指代“系统原有的 DNS 配置”。这在需要让某些特定域名（如内部局域网域名）走系统解析，而普通流量走加密 DNS 时非常有用。

```typescript
dns.setServers([
  '/my-company.com/system',   // 公司内网域名走系统 DNS (支持 VPN/局域网解析)
  'https://dns.google/dns-query' // 其他流量走 Google DoH
]);
```

---

## 🛠️ 进阶用法

### Resolver 实例 (独立配置)

如果你需要多个隔离的解析器（例如：一个用于调试，一个用于生产）：

```typescript
const customResolver = new dns.Resolver({
  timeout: 3000,   // 超时 (ms)
  tries: 2,        // 重试次数
  cacheSize: 500   // 该实例独享的缓存容量
});
customResolver.setServers(['1.1.1.1']);
const ips = await customResolver.promises.resolve4('github.com');
```

### Bootstrap IP / SNI 覆盖 (高级防劫持)

针对 DoH/DoT 服务器域名解析被劫持的情况，可以强制指定 IP 并手动设置 SNI：

**格式：`protocol://IP/path#OriginalHostname`**

```typescript
// 强制连接 8.8.8.8 的 DoH，TLS 校验使用 dns.google 做 SNI
dns.setServers(['https://8.8.8.8/dns-query#dns.google']);

// 强制连接 223.5.5.5 的 DoT，TLS 校验使用 dns.alidns.com
dns.setServers(['tls://223.5.5.5#dns.alidns.com']);
```

### 缓存策略说明

`CachePolicy` 枚举定义了内部缓存与后台缓存的行为模式：

| 策略 | 说明 |
| :--- | :--- |
| `FollowDnsTtl` (0) | **默认值**。严格遵循 DNS 服务器返回的 TTL 时间。 |
| `Bypass` (1) | 禁用所有缓存，每次解析都将发起网络请求。 |
| `StaleWhileRevalidate` (2) | **SWR**。如果缓存已过期但仍在宽限期内，立即返回过期数据并在后台异步刷新，刷新结果将在下次调用时生效。 |
| `StaleIfError` (3) | **SIE**。如果 DNS 请求失败或超时，且缓存中存在过期数据（在宽限期内），则返回该过期数据作为兜底。 |

---

## 🔌 深度原理

### 拦截技术栈
- **iOS (`NSURLProtocol`)**: 拦截所有 `NSURLSession` 请求。
  - **HTTPS & SNI**: 自动提取证书信息并以原始 Hostname 进行二次验证，确保 HTTPS 握手成功。
  - **Redirect Support**: 内置对 3xx 重定向的智能处理，自动跟随跳转（如 `http -> https`），行为与原生 fetch 保持一致。
- **Android (`OkHttp Hook`)**: 通过注入 `Dns` 代理类到 `OkHttpClient`。这是一种非侵入式的方式，能完美支持大部分基于 OkHttp 的库（包括 React Native 的 Networking 模块）。

### Rust 驱动
底层解析器基于 Rust 的 `hickory-resolver`，支持异步 I/O 及完整的 DNS 协议规范。

---

## 📄 许可证

ISC
