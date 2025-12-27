# API Compatibility Audit

This document outlines the API coverage of `react-native-nitro-dns` compared to the standard Node.js `dns` module.

## Summary

| Feature Category | Coverage | Notes |
| :--- | :--- | :--- |
| **Lookup** | ✅ Full | Supported: `family`, `hints`, `all`, `verbatim`, `order`. |
| **Resolution** | ✅ Full | All standard record types supported, including `TLSA`. |
| **Server Config** | ✅ Full | `getServers` / `setServers` (Global & Instance-based). |
| **Resolver Class** | ✅ Full | Supported: `timeout`, `tries`, `maxTimeout`, `setLocalAddress`. |
| **Promises API** | ✅ Full | `dns.promises` mirrors the full feature set. |
| **Utilities** | ✅ Full | `lookupService` implemented. |

---

## Detailed Breakdown

### 1. Lookup
| Method | Status | Notes |
| :--- | :--- | :--- |
| `dns.lookup(hostname, [options], callback)` | ✅ Supported | **Supported options**: `family` (0, 4, 6), `all` (boolean), `verbatim` (boolean), `hints` (number), `order` ('ipv4first', 'ipv6first', 'verbatim'). <br> **Constants**: `ADDRCONFIG`, `V4MAPPED`, `ALL` are exported. |
| `dns.setDefaultResultOrder(order)` | ✅ Supported | Sets the default sorting order for `dns.lookup`. |
| `dns.getDefaultResultOrder()` | ✅ Supported | |

### 2. Resolution Functions
| Method | Status | Notes |
| :--- | :--- | :--- |
| `dns.resolve(hostname, [rrtype], callback)` | ✅ Supported | Default `rrtype` is 'A'. Dispatches to specific methods. |
| `dns.resolve4(hostname, [options], callback)` | ✅ Supported | Supports `{ ttl: true }` option. |
| `dns.resolve6(hostname, [options], callback)` | ✅ Supported | Supports `{ ttl: true }` option. |
| `dns.resolveAny(hostname, callback)` | ✅ Supported | Full record details returned (A, AAAA, MX, NS, SOA, TXT, etc). |
| `dns.resolveCaa(hostname, callback)` | ✅ Supported | |
| `dns.resolveCname(hostname, callback)` | ✅ Supported | |
| `dns.resolveMx(hostname, callback)` | ✅ Supported | Includes `priority` and `exchange`. |
| `dns.resolveNaptr(hostname, callback)` | ✅ Supported | |
| `dns.resolveNs(hostname, callback)` | ✅ Supported | |
| `dns.resolvePtr(hostname, callback)` | ✅ Supported | |
| `dns.resolveSoa(hostname, callback)` | ✅ Supported | |
| `dns.resolveSrv(hostname, callback)` | ✅ Supported | Includes `priority`, `weight`, `port`, `name`. |
| `dns.resolveTxt(hostname, callback)` | ✅ Supported | |
| `dns.resolveTlsa(hostname, callback)` | ✅ Supported | Supported in both callback and promise APIs. |
| `dns.reverse(ip, callback)` | ✅ Supported | |

### 3. Server Management
| Method | Status | Notes |
| :--- | :--- | :--- |
| `dns.getServers()` | ✅ Supported | |
| `dns.setServers(servers)` | ✅ Supported | Supports custom protocols: `tls://`, `https://`, `quic://`. |

### 4. Resolver Class
| Class / Method | Status | Notes |
| :--- | :--- | :--- |
| `new dns.Resolver([options])` | ✅ Supported | Constructor supports `timeout`, `tries`. (`maxTimeout` is accepted but not applied by underlying resolver). |
| `resolver.cancel()` | ✅ Supported | |
| `resolver.getServers()` | ✅ Supported | |
| `resolver.setServers(servers)` | ✅ Supported | |
| `resolver.setLocalAddress(iv4, iv6)` | ✅ Supported | Supports binding to specific local IPv4/IPv6 addresses. |
| `resolver.resolve*()` | ✅ Supported | All `dns.resolve*` methods are available on `Resolver` instances. |

### 5. Promises API
The `dns.promises` API providing an identical set of methods as the callback-based API.

| Method | Status | Notes |
| :--- | :--- | :--- |
| `dns.promises.lookup(hostname, [options])` | ✅ Supported | |
| `dns.promises.lookupService(address, port)` | ✅ Supported | |
| `dns.promises.resolve(hostname, [rrtype])` | ✅ Supported | |
| `dns.promises.resolve4(hostname, [options])` | ✅ Supported | |
| `dns.promises.resolve6(hostname, [options])` | ✅ Supported | |
| `dns.promises.resolveAny(hostname)` | ✅ Supported | |
| `dns.promises.resolveCaa(hostname)` | ✅ Supported | |
| `dns.promises.resolveCname(hostname)` | ✅ Supported | |
| `dns.promises.resolveMx(hostname)` | ✅ Supported | |
| `dns.promises.resolveNaptr(hostname)` | ✅ Supported | |
| `dns.promises.resolveNs(hostname)` | ✅ Supported | |
| `dns.promises.resolvePtr(hostname)` | ✅ Supported | |
| `dns.promises.resolveSoa(hostname)` | ✅ Supported | |
| `dns.promises.resolveSrv(hostname)` | ✅ Supported | |
| `dns.promises.resolveTxt(hostname)` | ✅ Supported | |
| `dns.promises.resolveTlsa(hostname)` | ✅ Supported | |
| `dns.promises.reverse(ip)` | ✅ Supported | |
| `dns.promises.getServers()` | ✅ Supported | |
| `dns.promises.setServers(servers)` | ✅ Supported | |
| `new dns.promises.Resolver([options])` | ✅ Supported | Promise-based `Resolver` class. |

---

### 6. Module Constants
| Constant | Status | Notes |
| :--- | :--- | :--- |
| `dns.ADDRCONFIG` | ✅ Exported | |
| `dns.V4MAPPED` | ✅ Exported | |
| `dns.ALL` | ✅ Exported | |

### 7. Custom Extensions (Non-standard)
The following methods are added for enhanced React Native functionality:
| Method | Status | Description |
| :--- | :--- | :--- |
| `dns.setNativeInterceptionEnabled(bool)` | ✅ Added | Intercepts system-wide fetch/net requests. |
| `dns.setVerbose(bool)` | ✅ Added | Enables detailed logging of DNS operations. |
