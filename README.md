# react-native-nitro-dns 🚀

**react-native-nitro-dns** is a high-performance DNS module for React Native, powered by [Nitro Modules](https://nitro.margelo.com/) and a robust **Rust core**. It provides a Node.js-compatible `dns` API while extending modern privacy protocols (DoH, DoT, DoQ) and global network interception for mobile apps.

[![license](https://img.shields.io/badge/license-ISC-blue.svg)](https://github.com/iwater/rn-http-server/blob/main/LICENSE)
[![platform](https://img.shields.io/badge/platform-ios%20%7C%20android-lightgrey.svg)]()
[![compatibility](https://img.shields.io/badge/Node.js-100%25%20dns-green.svg)]()
[中文文档](./README_zh.md)

---

## ✨ Features

- 🚀 **Extreme Performance**: Core logic implemented in Rust with zero-copy JSI/C++ calls via Nitro Modules, bypassing the bridge.
- 📦 **Node.js Compatible**: Supports `lookup`, `resolve*`, `reverse`, `lookupService`, and `dns.promises` APIs.
- 🛡️ **Modern Privacy**:
  - **DoH** (DNS over HTTPS) - Custom paths supported.
  - **DoT** (DNS over TLS).
  - **DoQ** (DNS over QUIC).
- 🔌 **Global Interception**: Intercept all native network requests (`fetch`, `XMLHttpRequest`, etc.) and apply custom resolution rules.
- 🌐 **Cross-Platform Consistency**: Unified logic for Android (OkHttp Hook) and iOS (NSURLProtocol).
- 🧩 **Advanced Config**: SNI override (Bootstrap IP), timeout management, retry logic, and IPv4/IPv6 prioritization.

---

## 📥 Installation

```bash
# Using npm
npm install react-native-nitro-dns

# Using yarn
yarn add react-native-nitro-dns
```

### iOS Setup

iOS implementation is based on `NSURLProtocol`. It is automatically linked by Nitro, usually requiring no manual `AppDelegate` changes.

### Android Setup

To enable global OkHttp interception on Android, inject the Nitro factory in your `MainApplication.kt` (or `MainApplication.java`):

```kotlin
// MainApplication.kt (React Native 0.73+)
import com.nitrodns.NitroOkHttpClientFactory
import com.facebook.react.modules.network.OkHttpClientProvider

override fun onCreate() {
    super.onCreate()
    // Inject Nitro DNS factory for global fetch/XHR interception
    OkHttpClientProvider.setOkHttpClientFactory(NitroOkHttpClientFactory())
}
```

---

## 🚀 Quick Start

### 1. Domain Resolution (Node.js Style)

```typescript
import dns from 'react-native-nitro-dns';

// Using Promise APIs
const { address, family } = await dns.promises.lookup('google.com');

// Get all IP addresses via resolve4
const addresses = await dns.promises.resolve4('example.com');
```

### 2. Encrypted DNS Configuration

```typescript
// Configure DoH with custom path
dns.setServers(['https://dns.alidns.com/dns-query']);

// Configure DoT
dns.setServers(['tls://dns.google']);

// Hybrid configuration (Load balancing or failover)
dns.setServers(['quic://dns.nextdns.io', '8.8.8.8']);
```

### 3. Global Native Interception

Once enabled, all network requests (e.g., `fetch('https://...')`) will use Nitro DNS for resolution instead of system cache.

```typescript
// Enable interception
dns.setNativeInterceptionEnabled(true);

// This fetch call is now controlled by Nitro DNS (supporting DoH/DoT/etc.)
const res = await fetch('https://my-secure-api.com');
```

---

## 📖 API Reference

### `dns.promises` API

| Method | Description | Return Value | Advanced |
| :--- | :--- | :--- | :--- |
| `lookup` | Resolves a hostname | `{address, family}` | Supports `all`, `verbatim`, `family` via `options` |
| `resolve4` | Resolves IPv4 addresses | `string[]` | Set `{ttl: true}` for `[{address, ttl}]` |
| `resolve6` | Resolves IPv6 addresses | `string[]` | Set `{ttl: true}` for `[{address, ttl}]` |
| `resolveMx` | Resolves Mail Exchange records | `{exchange, priority}[]` | - |
| `resolveTxt` | Resolves Text records | `string[][]` | - |
| `resolveTlsa` | Resolves DANE fingerprints | `TLSA[]` | - |
| `lookupService` | Reverse lookup for IP/Port | `{hostname, service}` | - |

### Constants

Standard Node.js constants for `lookup` hints:
- `ADDRCONFIG`
- `V4MAPPED`
- `ALL`

---

### 4. Domain Routing (dnsmasq Style)

Route specific domains to specific servers while using defaults for others.

**Format: `/domain/server_url`**

```typescript
dns.setServers([
  '/google.com/8.8.8.8',      // Use 8.8.8.8 for google.com and its subdomains
  '/corp.local/10.0.0.1',     // Use internal DNS for local corp domains
  '1.1.1.1'                   // Default server for everyone else
]);
```

### 5. System DNS Fallback (`system`)

Use the `system` keyword to refer to the device's original DNS configuration. This is useful for VPN/Intranet routing.

```typescript
dns.setServers([
  '/my-company.com/system',      // Use system resolver for company intranet
  'https://dns.google/dns-query' // Use Google DoH for everything else
]);
```

---

## 🛠️ Advanced Usage

### Independent Resolver Instances

If you need isolated DNS configurations (e.g., for different environments):

```typescript
const customResolver = new dns.Resolver({
  timeout: 3000,   // ms
  tries: 2         // retry attempts
});
customResolver.setServers(['1.1.1.1']);
const ips = await customResolver.promises.resolve4('github.com');
```

### Bootstrap IP / SNI Override

To prevent DNS hijacking of the DoH/DoT server itself, specify a hardcoded IP and SNI:

**Format: `protocol://IP/path#OriginalHostname`**

```typescript
// Force connect to 8.8.8.8 for DoH, using dns.google for SNI validation
dns.setServers(['https://8.8.8.8/dns-query#dns.google']);

// Force connect to 223.5.5.5 for DoT, using dns.alidns.com for SNI
dns.setServers(['tls://223.5.5.5#dns.alidns.com']);
```

---

## 🔌 Architecture

```mermaid
graph LR
    JS[JavaScript App] -- JSI --> Nitro[Nitro Modules C++]
    Nitro -- FFI --> Rust[Rust Core]
    Rust -- System --> OS[iOS/Android Network Stack]
```

### Interception Logic
- **iOS (`NSURLProtocol`)**: Intercepts all `NSURLSession` tasks. Handles HTTPS SNI validation by extracting certificates and verifying against the original hostname even when connecting via raw IP. Includes smart 3xx redirect handling.
- **Android (`OkHttp Hook`)**: Injects a custom `Dns` proxy into the `OkHttpClient`. This provides non-invasive support for most networking libraries (including React Native's core Networking).

---

## 📄 License

ISC
