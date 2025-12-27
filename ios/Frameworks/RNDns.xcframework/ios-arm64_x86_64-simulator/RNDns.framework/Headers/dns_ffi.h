#pragma once
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
  char *data;
  char *error;
} DnsResult;

void dns_free_result(DnsResult res);

// Global Config
DnsResult dns_set_servers(const char *servers_json);
DnsResult dns_get_servers();

// Resolver Management
int32_t dns_resolver_new(const char *servers_json);
void dns_resolver_delete(int32_t id);
DnsResult dns_resolver_get_servers(int32_t id);
void dns_resolver_set_local_address(int32_t id, const char *v4, const char *v6);

// System Lookup
// System Lookup
DnsResult dns_lookup_sync(const char *hostname, int family);
DnsResult dns_lookup_service(const char *ip, uint16_t port);

// Resolution Methods (id -1 for global/default)
DnsResult dns_resolve_ipv4(int32_t id, const char *hostname);
DnsResult dns_resolve_ipv6(int32_t id, const char *hostname);
DnsResult dns_resolve_mx(int32_t id, const char *hostname);
DnsResult dns_resolve_txt(int32_t id, const char *hostname);
DnsResult dns_resolve_ns(int32_t id, const char *hostname);
DnsResult dns_resolve_cname(int32_t id, const char *hostname);
DnsResult dns_resolve_soa(int32_t id, const char *hostname);
DnsResult dns_resolve_srv(int32_t id, const char *hostname);
DnsResult dns_resolve_caa(int32_t id, const char *hostname);
DnsResult dns_resolve_tlsa(int32_t id, const char *hostname);
DnsResult dns_resolve_naptr(int32_t id, const char *hostname);
DnsResult dns_resolve_ptr(int32_t id, const char *hostname);
DnsResult dns_resolve_any(int32_t id, const char *hostname);
DnsResult dns_reverse(int32_t id, const char *ip);

#ifdef __cplusplus
}
#endif
