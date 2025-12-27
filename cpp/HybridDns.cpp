#ifdef __ANDROID__
#include <fbjni/fbjni.h>
#include <jni.h>
#endif
#ifdef __APPLE__
extern "C" void nitro_dns_set_interception_enabled_ios(bool enabled);
#endif
#include "HybridDns.hpp"
#include "dns_ffi.h"
#include <functional>
#include <iostream>
#include <stdexcept>
#include <thread>
#include <vector>

#ifdef __ANDROID__
#include <android/log.h>
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, "NitroDns", __VA_ARGS__)
#endif

namespace margelo::nitro::nitro_dns {

#ifdef __ANDROID__
static JNIEnv *getJNIEnv() { return facebook::jni::Environment::current(); }
#endif

// Helper to handle DnsResult
std::string handle_result(DnsResult res) {
  std::string output;
  if (res.error) {
    std::string err(res.error);
    dns_free_result(res);
    throw std::runtime_error(err);
  }
  if (res.data) {
    output = std::string(res.data);
  } else {
    output = "[]";
  }
  dns_free_result(res);
  return output;
}

std::shared_ptr<Promise<std::string>>
runAsync(std::function<std::string()> func) {
  auto promise = Promise<std::string>::create();
  std::thread([promise, func]() {
    try {
      std::string result = func();
      promise->resolve(result);
    } catch (const std::exception &e) {
      promise->reject(std::make_exception_ptr(std::runtime_error(e.what())));
    } catch (...) {
      promise->reject(std::make_exception_ptr(
          std::runtime_error("Unknown error in DNS operation")));
    }
  }).detach();
  return promise;
}

// -------------------------------------------------------------------------
// HybridResolver Implementation
// -------------------------------------------------------------------------

HybridResolver::~HybridResolver() { dns_resolver_delete(id_); }

void HybridResolver::cancel() {
  // Cancellation not implemented in Rust FFI yet.
  // Hickory resolver handles cancellation by dropping future, but we detach
  // threads. To support cancellation, we'd need a way to signal the thread. For
  // now, no-op or throw "Not Implemented". Node 20+ supports cancel.
}

std::string HybridResolver::getServers() {
  DnsResult res = dns_resolver_get_servers(id_);
  try {
    return handle_result(res);
  } catch (...) {
    return "[]"; // Or throw?
  }
}

void HybridResolver::setServers(const std::string & /*servers*/) {
  // Per-resolver setServers not implemented in FFI yet properly (only global
  // supports setServers logic in simple implementation) Actually Rust
  // `dns_resolver_new` takes config, but we don't have update function. We
  // should throw or log. Node.js Resolver.setServers is supported.
  throw std::runtime_error("setServers is not supported on Resolver instances "
                           "yet (re-create Resolver with new config instead)");
}

// All resolve methods forward to runAsync with id_
#define RESOLVER_METHOD(Name, FFI_Func)                                        \
  std::shared_ptr<Promise<std::string>> HybridResolver::Name(                  \
      const std::string &hostname) {                                           \
    int32_t id = id_;                                                          \
    return runAsync([id, hostname]() {                                         \
      DnsResult res = FFI_Func(id, hostname.c_str());                          \
      return handle_result(res);                                               \
    });                                                                        \
  }

RESOLVER_METHOD(resolveipv4, dns_resolve_ipv4)
RESOLVER_METHOD(resolveipv6, dns_resolve_ipv6)
RESOLVER_METHOD(resolveMx, dns_resolve_mx)
RESOLVER_METHOD(resolveTxt, dns_resolve_txt)
RESOLVER_METHOD(resolveCname, dns_resolve_cname)
RESOLVER_METHOD(resolveNs, dns_resolve_ns)
RESOLVER_METHOD(resolveSoa, dns_resolve_soa)
RESOLVER_METHOD(resolveSrv, dns_resolve_srv)
RESOLVER_METHOD(resolveCaa, dns_resolve_caa)
RESOLVER_METHOD(resolveNaptr, dns_resolve_naptr)
RESOLVER_METHOD(resolvePtr, dns_resolve_ptr)
RESOLVER_METHOD(resolveTlsa, dns_resolve_tlsa)
RESOLVER_METHOD(resolveAny, dns_resolve_any)

std::shared_ptr<Promise<std::string>>
HybridResolver::reverse(const std::string &ip) {
  int32_t id = id_;
  return runAsync([id, ip]() {
    DnsResult res = dns_reverse(id, ip.c_str());
    return handle_result(res);
  });
}

void HybridResolver::setLocalAddress(const std::string &v4,
                                     const std::string &v6) {
  dns_resolver_set_local_address(id_, v4.c_str(), v6.c_str());
}
// Base64 helper if needed, but resolveTxt supports it naturally via buffer?
// Node: resolveTxt returns arrays of strings.
// If we need base64 specific for other types, fine.
// For now alias to resolveTxt? No, base64txt is not standard node.
std::shared_ptr<Promise<std::string>>
HybridResolver::resolveBase64Txt(const std::string &hostname) {
  return resolveTxt(hostname); // Placeholder
}

// -------------------------------------------------------------------------
// HybridDns Implementation
// -------------------------------------------------------------------------

std::shared_ptr<HybridNitroResolverSpec>
HybridDns::createResolver(const std::optional<std::string> &config) {
  const char *cfg = config.has_value() ? config->c_str() : "{}";
  int32_t id = dns_resolver_new(cfg);
  if (id < 0) {
    throw std::runtime_error("Failed to create resolver");
  }
  return std::make_shared<HybridResolver>(id);
}

std::string HybridDns::getServers() {
  DnsResult res = dns_get_servers();
  return handle_result(res);
}

void HybridDns::setServers(const std::string &servers) {
  DnsResult res = dns_set_servers(servers.c_str());
  handle_result(res);
}

std::string HybridDns::lookup(const std::string &hostname, double family) {
  DnsResult res = dns_lookup_sync(hostname.c_str(), (int)family);
  return handle_result(res);
}

// Global resolve calls use id = -1
#define GLOBAL_METHOD(Name, FFI_Func)                                          \
  std::shared_ptr<Promise<std::string>> HybridDns::Name(                       \
      const std::string &hostname) {                                           \
    return runAsync([hostname]() {                                             \
      DnsResult res = FFI_Func(-1, hostname.c_str());                          \
      return handle_result(res);                                               \
    });                                                                        \
  }

GLOBAL_METHOD(resolve4, dns_resolve_ipv4)
GLOBAL_METHOD(resolve6, dns_resolve_ipv6)
GLOBAL_METHOD(resolveMx, dns_resolve_mx)
GLOBAL_METHOD(resolveTxt, dns_resolve_txt)
GLOBAL_METHOD(resolveCname, dns_resolve_cname)
GLOBAL_METHOD(resolveNs, dns_resolve_ns)
GLOBAL_METHOD(resolveSoa, dns_resolve_soa)
GLOBAL_METHOD(resolveSrv, dns_resolve_srv)
GLOBAL_METHOD(resolveCaa, dns_resolve_caa)
GLOBAL_METHOD(resolveNaptr, dns_resolve_naptr)
GLOBAL_METHOD(resolvePtr, dns_resolve_ptr)
GLOBAL_METHOD(resolveTlsa, dns_resolve_tlsa)
GLOBAL_METHOD(resolveAny, dns_resolve_any)

std::shared_ptr<Promise<std::string>>
HybridDns::reverse(const std::string &ip) {
  return runAsync([ip]() {
    DnsResult res = dns_reverse(-1, ip.c_str());
    return handle_result(res);
  });
}

std::shared_ptr<Promise<std::string>>
HybridDns::lookupService(const std::string &address, double port) {
  uint16_t p = (uint16_t)port;
  return runAsync([address, p]() {
    DnsResult res = dns_lookup_service(address.c_str(), p);
    return handle_result(res);
  });
}

void HybridDns::setNativeInterceptionEnabled(bool enabled) {
#ifdef __ANDROID__
  JNIEnv *env = getJNIEnv();
  if (env == nullptr)
    return;
  jclass clazz = env->FindClass("com/nitrodns/NitroDnsProvider");
  if (clazz == nullptr)
    return;
  jmethodID methodId = env->GetStaticMethodID(clazz, "setEnabled", "(Z)V");
  if (methodId == nullptr)
    return;
#ifdef __ANDROID__
  LOGI("Setting native interception enabled: %d", enabled);
#endif
  env->CallStaticVoidMethod(clazz, methodId, (jboolean)enabled);
#endif
#ifdef __APPLE__
  nitro_dns_set_interception_enabled_ios(enabled);
#endif
}

extern "C" void dns_set_verbose(bool enabled);

void HybridDns::setVerbose(bool enabled) { dns_set_verbose(enabled); }

} // namespace margelo::nitro::nitro_dns

#ifdef __ANDROID__
extern "C" JNIEXPORT jstring JNICALL Java_com_nitrodns_NitroDns_resolve(
    JNIEnv *env, jclass /*clazz*/, jstring hostname) {
  using namespace margelo::nitro::nitro_dns;
  const char *host = env->GetStringUTFChars(hostname, nullptr);
  if (host == nullptr)
    return env->NewStringUTF("[]");

  std::string h(host);
  env->ReleaseStringUTFChars(hostname, host);

  try {
    DnsResult res = dns_lookup_sync(h.c_str(), 0);
    std::string output = handle_result(res);
    return env->NewStringUTF(output.c_str());
  } catch (const std::exception &e) {
    return env->NewStringUTF("[]");
  } catch (...) {
    return env->NewStringUTF("[]");
  }
}
#endif
