use libc::{c_char, c_int};
use std::ffi::{CStr, CString};
use std::ptr;
use std::sync::{Arc, RwLock};
use std::collections::{HashMap, BTreeMap};
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{Instant, Duration};
use tokio::runtime::Runtime;
use hickory_resolver::TokioAsyncResolver;
use hickory_resolver::proto::rr::RecordType;
use hickory_resolver::config::{ResolverConfig, ResolverOpts, Protocol};
use std::net::SocketAddr;
use lazy_static::lazy_static;
use serde::Serialize;
use std::sync::Once;
use tracing_subscriber::FmtSubscriber;

static INIT_LOGGER: Once = Once::new();

#[no_mangle]
pub extern "C" fn dns_set_verbose(enabled: bool) {
    if enabled {
        INIT_LOGGER.call_once(|| {
            use tracing_subscriber::EnvFilter;
            let filter = EnvFilter::new("rust_c_dns=trace,hickory_resolver=debug,hickory_proto=debug,info");
            
            let subscriber = FmtSubscriber::builder()
                .with_env_filter(filter)
                .with_ansi(false)
                .finish();
            let _ = tracing::subscriber::set_global_default(subscriber);
            tracing::info!("NitroDns: Verbose logging enabled (TRACE level)");
        });
    }
}

// Global Runtime and storage for resolvers
lazy_static! {
    static ref RUNTIME: Runtime = Runtime::new().expect("Failed to create Tokio runtime");
    static ref RESOLVER_STORE: RwLock<HashMap<i32, (Arc<TokioAsyncResolver>, ResolverConfig)>> = RwLock::new(HashMap::new());
    static ref NEXT_ID: AtomicI32 = AtomicI32::new(1);
    static ref GLOBAL_CONFIG: RwLock<Option<ResolverConfig>> = RwLock::new(None);
    // Domain routing table: maps domain suffix (e.g., "google.com") to a specific Resolver
    static ref ROUTING_TABLE: RwLock<BTreeMap<String, Arc<TokioAsyncResolver>>> = RwLock::new(BTreeMap::new());
    // Cached default resolver (system DNS) to avoid creating new instance each time
    static ref DEFAULT_RESOLVER: RwLock<Option<Arc<TokioAsyncResolver>>> = RwLock::new(None);
    
    // Secondary cache for Stale-While-Revalidate and Stale-If-Error
    // Key: "id:hostname:rrtype", Value: (JSON, Expiry, StaleExpiry)
    static ref STALE_CACHE: RwLock<HashMap<String, (String, Instant, Instant)>> = RwLock::new(HashMap::new());
}

#[repr(i32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CachePolicy {
    FollowDnsTtl = 0,
    Bypass = 1,
    StaleWhileRevalidate = 2,
    StaleIfError = 3,
}

static GLOBAL_OPTS_CACHE_SIZE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(32); // default 32
static GLOBAL_CACHE_POLICY: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0); // FollowDnsTtl
static GLOBAL_STALE_TTL: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(3600); // 1 hour stale grace period

fn find_routing_resolver(hostname: &str) -> Option<Arc<TokioAsyncResolver>> {
    let hostname = hostname.trim_end_matches('.');
    let guard = ROUTING_TABLE.read().unwrap();
    let mut best_match: Option<(String, Arc<TokioAsyncResolver>)> = None;

    for (domain, resolver) in guard.iter() {
        let domain_cmp = domain.trim_end_matches('.');
        // Check if hostname matches domain exactly or is a subdomain
        if hostname == domain_cmp || hostname.ends_with(&format!(".{}", domain_cmp)) {
            if best_match.as_ref().map_or(true, |(best_domain, _)| domain_cmp.len() > best_domain.len()) {
                best_match = Some((domain_cmp.to_string(), resolver.clone()));
            }
        }
    }
    best_match.map(|(_, r)| r)
}

/// Get or create the cached default resolver (system DNS)
fn get_or_create_default_resolver() -> Arc<TokioAsyncResolver> {
    // First, try to read cached resolver
    {
        let guard = DEFAULT_RESOLVER.read().unwrap();
        if let Some(resolver) = &*guard {
            return resolver.clone();
        }
    }
    
    // Create and cache the default resolver
    let mut guard = DEFAULT_RESOLVER.write().unwrap();
    // Double-check after acquiring write lock
    if let Some(resolver) = &*guard {
        return resolver.clone();
    }
    
    let resolver = Arc::new(RUNTIME.block_on(async {
        let (cfg, mut opts) = if let Ok((cfg, opts)) = hickory_resolver::system_conf::read_system_conf() {
            (cfg, opts)
        } else {
            (ResolverConfig::default(), ResolverOpts::default())
        };
        opts.cache_size = GLOBAL_OPTS_CACHE_SIZE.load(Ordering::SeqCst);
        TokioAsyncResolver::tokio(cfg, opts)
    }));
    *guard = Some(resolver.clone());
    resolver
}

fn get_resolver_by_id(id: i32, hostname: Option<&str>) -> Result<Arc<TokioAsyncResolver>, String> {
    if id == -1 {
        // Global resolver routing
        if let Some(host) = hostname {
            if let Some(resolver) = find_routing_resolver(host) {
                tracing::info!("NitroDns: Routed hostname '{}' to domain-specific resolver", host);
                return Ok(resolver);
            }
        }

        // Check for custom global config
        let guard = GLOBAL_CONFIG.read().unwrap();
        if let Some(cfg) = &*guard {
            // Custom global config - create resolver with it
            return Ok(Arc::new(RUNTIME.block_on(async { 
                TokioAsyncResolver::tokio(cfg.clone(), ResolverOpts::default()) 
            })));
        }
        drop(guard);
        
        // Use cached default resolver (system DNS)
        return Ok(get_or_create_default_resolver());
    }
    
    let store = RESOLVER_STORE.read().unwrap();
    store.get(&id)
        .map(|(resolver, _config)| resolver.clone())
        .ok_or_else(|| "Resolver not found".to_string())
}

// -------------------------------------------------------------------------
// Structs for Serialization
// -------------------------------------------------------------------------
#[repr(C)]
pub struct DnsResult {
    pub data: *mut c_char,
    pub error: *mut c_char,
}
fn create_success_result(json: String) -> DnsResult {
    DnsResult { data: CString::new(json).unwrap_or_default().into_raw(), error: ptr::null_mut() }
}
fn create_error_result(err: String) -> DnsResult {
    DnsResult { data: ptr::null_mut(), error: CString::new(err).unwrap_or_default().into_raw() }
}
#[no_mangle]
pub extern "C" fn dns_free_result(res: DnsResult) {
    if !res.data.is_null() { unsafe { let _ = CString::from_raw(res.data); } }
    if !res.error.is_null() { unsafe { let _ = CString::from_raw(res.error); } }
}

// -------------------------------------------------------------------------
// Config Management
// -------------------------------------------------------------------------
struct ParsedServer {
    addr: SocketAddr,
    protocol: Protocol,
    tls_dns_name: Option<String>,
    domain: Option<String>, // Optional domain for routing (dnsmasq style)
    is_system: bool,        // Flag for system DNS
}


fn parse_server_str(s: &str) -> std::result::Result<ParsedServer, String> {
    let s = s.trim();

    // 0. Extract domain routing prefix (e.g., /google.com/8.8.8.8)
    let (s, domain) = if s.starts_with('/') {
        if let Some(second_slash) = s[1..].find('/') {
            let domain = &s[1..second_slash + 1];
            let rest = &s[second_slash + 2..];
            (rest.trim(), Some(domain.to_string()))
        } else {
            (s, None)
        }
    } else {
        (s, None)
    };

    // 0.5 Extract fragment (Bootstrap SNI)
    let (s_without_fragment, bootstrap_hostname) = if let Some(pos) = s.find('#') {
        (&s[..pos], Some(&s[pos+1..]))
    } else {
        (s, None)
    };
    let s = s_without_fragment;

    // 1. Detect Protocol
    let (protocol, rest, default_port) = if s.starts_with("tcp://") {
        (Protocol::Tcp, &s[6..], 53)
    } else if s.starts_with("udp://") {
        (Protocol::Udp, &s[6..], 53)
    } else if s.starts_with("https://") {
        (Protocol::Https, &s[8..], 443)
    } else if s.starts_with("http://") {
        (Protocol::Https, &s[7..], 80)
    } else if s.starts_with("tls://") {
        (Protocol::Tls, &s[6..], 853)
    } else if s.starts_with("quic://") {
        (Protocol::Quic, &s[7..], 853)
    } else {
        (Protocol::Udp, s, 53)
    };

    if rest == "system" {
        return Ok(ParsedServer {
            addr: "127.0.0.1:0".parse().unwrap(),
            protocol: Protocol::Udp,
            tls_dns_name: None,
            domain,
            is_system: true,
        });
    }

    // 2. Parse Address or Hostname - Strip path and fragment
    let rest = rest.split('/').next().unwrap_or(rest).split('#').next().unwrap_or(rest);

    if let Ok(addr) = rest.parse::<SocketAddr>() {
        return Ok(ParsedServer {
            addr,
            protocol,
            tls_dns_name: if protocol == Protocol::Tls || protocol == Protocol::Https || protocol == Protocol::Quic {
                bootstrap_hostname.map(|h| h.to_string())
            } else {
                None
            },
            domain,
            is_system: false,
        });
    }

    // B. IP without port (handle potential brackets [2001:...])
    let rest_for_ip = if rest.starts_with('[') && rest.ends_with(']') {
        &rest[1..rest.len()-1]
    } else {
        rest
    };

    if let Ok(ip) = rest_for_ip.parse::<std::net::IpAddr>() {
        let addr = SocketAddr::new(ip, default_port);
        return Ok(ParsedServer {
            addr,
            protocol,
            tls_dns_name: if protocol == Protocol::Tls || protocol == Protocol::Https || protocol == Protocol::Quic {
                bootstrap_hostname.map(|h| h.to_string())
            } else {
                None
            },
            domain,
            is_system: false,
        });
    }

    // C. Hostname (e.g. "dns.google" or "dns.google:853")
    // If it contains a colon, it might be Host:Port
    let (hostname, port) = if let Some(pos) = rest.rfind(':') {
        let h = &rest[..pos];
        let p = &rest[pos+1..];
        
        // Handle [IP]:Port case for hostname resolution if it failed SocketAddr parse
        let h = if h.starts_with('[') && h.ends_with(']') {
            &h[1..h.len()-1]
        } else {
            h
        };

        if let Ok(port_num) = p.parse::<u16>() {
            (h, port_num)
        } else {
            (rest, default_port)
        }
    } else {
        (rest, default_port)
    };

    // Resolve hostname to IP
    match dns_lookup::lookup_host(hostname) {
        Ok(ips) => {
            if let Some(ip) = ips.first() {
                let addr = SocketAddr::new(*ip, port);
                return Ok(ParsedServer {
                    addr,
                    protocol,
                    tls_dns_name: if protocol == Protocol::Tls || protocol == Protocol::Https || protocol == Protocol::Quic {
                        Some(bootstrap_hostname.map(|h| h.to_string()).unwrap_or_else(|| hostname.to_string()))
                    } else {
                        None
                    },
                    domain,
                    is_system: false,
                });
            }
            Err(format!("Could not resolve hostname: {}", hostname))
        }
        Err(e) => Err(format!("Failed to resolve hostname '{}': {}", hostname, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hickory_resolver::config::Protocol;
    use std::sync::Mutex;

    lazy_static! {
        static ref UNIT_TEST_LOCK: Mutex<()> = Mutex::new(());
    }

    fn acquire_lock() -> std::sync::MutexGuard<'static, ()> {
        UNIT_TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn test_parse_server_str() {
        // Test TCP
        let res = parse_server_str("tcp://8.8.8.8").unwrap();
        assert_eq!(res.protocol, Protocol::Tcp);
        assert_eq!(res.addr.to_string(), "8.8.8.8:53");

        let res = parse_server_str("tcp://8.8.8.8:853").unwrap();
        assert_eq!(res.protocol, Protocol::Tcp);
        assert_eq!(res.addr.to_string(), "8.8.8.8:853");

        // Test UDP
        let res = parse_server_str("udp://1.1.1.1").unwrap();
        assert_eq!(res.protocol, Protocol::Udp);
        assert_eq!(res.addr.to_string(), "1.1.1.1:53");

        let res = parse_server_str("1.2.3.4").unwrap();
        assert_eq!(res.protocol, Protocol::Udp);
        assert_eq!(res.addr.to_string(), "1.2.3.4:53");

        // Test IPv6
        let res = parse_server_str("udp://[2001:4860:4860::8888]").unwrap();
        assert_eq!(res.protocol, Protocol::Udp);
        assert_eq!(res.addr.to_string(), "[2001:4860:4860::8888]:53");

        let res = parse_server_str("tcp://[::1]:5353").unwrap();
        assert_eq!(res.protocol, Protocol::Tcp);
        assert_eq!(res.addr.to_string(), "[::1]:5353");
        
        // Test Bare IPv6 (if we support it)
        let res = parse_server_str("::1").unwrap();
        assert_eq!(res.protocol, Protocol::Udp);
        assert_eq!(res.addr.to_string(), "[::1]:53");

        // Test DoH
        let res = parse_server_str("https://8.8.8.8").unwrap();
        assert_eq!(res.protocol, Protocol::Https);
        assert_eq!(res.addr.to_string(), "8.8.8.8:443");

        let res = parse_server_str("http://1.1.1.1").unwrap();
        assert_eq!(res.protocol, Protocol::Https);
        assert_eq!(res.addr.to_string(), "1.1.1.1:80");

        // Test DoT with Hostname
        // Note: dns_lookup::lookup_host will resolve live, so we check if it succeeds
        let res = parse_server_str("tls://dns.google").unwrap();
        assert_eq!(res.protocol, Protocol::Tls);
        assert!(res.addr.to_string().ends_with(":853"));
        assert_eq!(res.tls_dns_name, Some("dns.google".to_string()));

        // Test DoT with IP (should have no SNI by default if we haven't mapped)
        let res = parse_server_str("tls://8.8.8.8").unwrap();
        assert_eq!(res.protocol, Protocol::Tls);
        assert_eq!(res.addr.to_string(), "8.8.8.8:853");
        assert_eq!(res.tls_dns_name, None);

        // Test DoQ
        let res = parse_server_str("quic://dns.google").unwrap();
        assert_eq!(res.protocol, Protocol::Quic);
        assert!(res.addr.to_string().ends_with(":853"));
        assert_eq!(res.tls_dns_name, Some("dns.google".to_string()));

        let res = parse_server_str("quic://1.1.1.1").unwrap();
        assert_eq!(res.protocol, Protocol::Quic);
        assert_eq!(res.addr.to_string(), "1.1.1.1:853");
        assert_eq!(res.tls_dns_name, None);

        // Test URIs with paths
        let res = parse_server_str("https://8.8.8.8/dns-query").unwrap();
        assert_eq!(res.protocol, Protocol::Https);
        assert_eq!(res.addr.to_string(), "8.8.8.8:443");

        let res = parse_server_str("http://1.1.1.1/resolve").unwrap();
        assert_eq!(res.protocol, Protocol::Https);
        assert_eq!(res.addr.to_string(), "1.1.1.1:80");

        let res = parse_server_str("https://dns.google/dns-query").unwrap();
        assert_eq!(res.protocol, Protocol::Https);
        assert_eq!(res.tls_dns_name, Some("dns.google".to_string()));

        // Test Bootstrap IP (SNI Override)
        let res = parse_server_str("https://8.8.8.8/dns-query#dns.google").unwrap();
        assert_eq!(res.protocol, Protocol::Https);
        assert_eq!(res.addr.to_string(), "8.8.8.8:443");
        assert_eq!(res.tls_dns_name, Some("dns.google".to_string()));

        let res = parse_server_str("tls://1.1.1.1#cloudflare-dns.com").unwrap();
        assert_eq!(res.protocol, Protocol::Tls);
        assert_eq!(res.addr.to_string(), "1.1.1.1:853");
        assert_eq!(res.tls_dns_name, Some("cloudflare-dns.com".to_string()));
    }

    #[test]
    fn test_parse_domain_routing() {
        let res = parse_server_str("/google.com/8.8.8.8").unwrap();
        assert_eq!(res.domain, Some("google.com".to_string()));
        assert_eq!(res.addr, "8.8.8.8:53".parse().unwrap());

        let res = parse_server_str("/corp/https://10.0.0.1/dns-query").unwrap();
        assert_eq!(res.domain, Some("corp".to_string()));
        assert_eq!(res.addr, "10.0.0.1:443".parse().unwrap());
        assert_eq!(res.protocol, Protocol::Https);
    }

    #[test]
    fn test_routing_logic() {
        let _lock = acquire_lock();
        // Clear and setup routing table for test
        {
            let mut guard = ROUTING_TABLE.write().unwrap();
            guard.clear();
            let cfg = ResolverConfig::new();
            let r = RUNTIME.block_on(async { TokioAsyncResolver::tokio(cfg, ResolverOpts::default()) });
            guard.insert("google.com".to_string(), Arc::new(r));
            
            let cfg2 = ResolverConfig::new();
            let r2 = RUNTIME.block_on(async { TokioAsyncResolver::tokio(cfg2, ResolverOpts::default()) });
            guard.insert("api.google.com".to_string(), Arc::new(r2));
        }

        // Test matches
        assert!(find_routing_resolver("www.google.com").is_some());
        assert!(find_routing_resolver("google.com").is_some());
        
        // Test longest match
        assert!(find_routing_resolver("api.google.com").is_some());
        assert!(find_routing_resolver("sub.api.google.com").is_some());

        // Test no matches
        assert!(find_routing_resolver("notgoogle.com").is_none());
        assert!(find_routing_resolver("api-google.com").is_none());
    }

    #[test]
    fn test_full_routing_flow() {
        let _lock = acquire_lock();
        let json = r#"["/google.com/8.8.8.8", "/corp/10.0.0.1", "1.1.1.1"]"#;
        let c_json = CString::new(json).unwrap();
        let res = dns_set_servers(c_json.as_ptr());
        assert!(res.error.is_null());
        dns_free_result(res);

        // Verify routing table has 2 entries
        {
            let guard = ROUTING_TABLE.read().unwrap();
            assert_eq!(guard.len(), 2);
            assert!(guard.contains_key("google.com"));
            assert!(guard.contains_key("corp"));
        }

        // Verify global config has 1 server (1.1.1.1)
        {
            let guard = GLOBAL_CONFIG.read().unwrap();
            let cfg = guard.as_ref().unwrap();
            assert_eq!(cfg.name_servers().len(), 1);
            assert_eq!(cfg.name_servers()[0].socket_addr.ip().to_string(), "1.1.1.1");
        }

        // Verify selection
        assert!(find_routing_resolver("www.google.com").is_some());
        assert!(find_routing_resolver("internal.corp").is_some());
        assert!(find_routing_resolver("bing.com").is_none());
    }

    #[test]
    fn test_system_routing() {
        let _lock = acquire_lock();
        let json = r#"["/system.local/system", "8.8.8.8"]"#;
        let c_json = CString::new(json).unwrap();
        let res = dns_set_servers(c_json.as_ptr());
        assert!(res.error.is_null());
        dns_free_result(res);

        // Verify routing table has entry for system.local
        {
            let guard = ROUTING_TABLE.read().unwrap();
            assert!(guard.contains_key("system.local"));
        }

        // Verify selection
        assert!(find_routing_resolver("my.system.local").is_some());
        assert!(find_routing_resolver("google.com").is_none());
    }

    #[test]
    fn test_stale_cache_logic() {
        let _lock = acquire_lock();
        let key = "-1:test.com:A";
        let json_data = r#"[{"address":"1.1.1.1","ttl":60,"family":4}]"#;
        let now = Instant::now();
        
        // 1. Test Fresh Hit
        {
            let mut guard = STALE_CACHE.write().unwrap();
            guard.clear();
            guard.insert(key.to_string(), (json_data.to_string(), now + Duration::from_secs(60), now + Duration::from_secs(3600)));
        }
        
        GLOBAL_CACHE_POLICY.store(CachePolicy::StaleWhileRevalidate as i32, Ordering::SeqCst);
        
        // Should return cached data and NOT call the resolver
        let result = resolve_helper::<_, IpRecord>(-1, Some("test.com"), "A", |_| async {
            use hickory_resolver::error::ResolveErrorKind;
            Err(hickory_resolver::error::ResolveError::from(ResolveErrorKind::Msg("Should not be called".into())))
        });
        
        let res_json = unsafe { CStr::from_ptr(result.data).to_string_lossy() };
        assert_eq!(res_json, json_data);
        dns_free_result(result);

        // 2. Test Stale Hit (Expired but within grace)
        {
            let mut guard = STALE_CACHE.write().unwrap();
            guard.insert(key.to_string(), (json_data.to_string(), now - Duration::from_secs(1), now + Duration::from_secs(3600)));
        }
        
        let result = resolve_helper::<_, IpRecord>(-1, Some("test.com"), "A", |_| async {
            use hickory_resolver::error::ResolveErrorKind;
            Err(hickory_resolver::error::ResolveError::from(ResolveErrorKind::Msg("Should not be called".into())))
        });
        
        let res_json = unsafe { CStr::from_ptr(result.data).to_string_lossy() };
        assert_eq!(res_json, json_data);
        dns_free_result(result);

        // 3. Test Stale-If-Error
        GLOBAL_CACHE_POLICY.store(CachePolicy::StaleIfError as i32, Ordering::SeqCst);
        {
            let mut guard = STALE_CACHE.write().unwrap();
            // Data is expired
            guard.insert(key.to_string(), (json_data.to_string(), now - Duration::from_secs(100), now + Duration::from_secs(3600)));
        }

        // resolve_helper should try to fetch, fail, and then return stale data
        let result = resolve_helper::<_, IpRecord>(-1, Some("test.com"), "A", |_| async {
            use hickory_resolver::error::ResolveErrorKind;
            Err(hickory_resolver::error::ResolveError::from(ResolveErrorKind::Msg("Network error".into())))
        });
        
        let res_json = unsafe { CStr::from_ptr(result.data).to_string_lossy() };
        assert_eq!(res_json, json_data);
        dns_free_result(result);
        
        // 4. Test Bypass
        GLOBAL_CACHE_POLICY.store(CachePolicy::Bypass as i32, Ordering::SeqCst);
        let result = resolve_helper::<_, IpRecord>(-1, Some("test.com"), "A", |_| async {
            use hickory_resolver::error::ResolveErrorKind;
            Err(hickory_resolver::error::ResolveError::from(ResolveErrorKind::Msg("Should fail".into())))
        });
        
        let error = unsafe { CStr::from_ptr(result.error).to_string_lossy() };
        assert!(error.contains("Should fail"));
        dns_free_result(result);
    }
}

#[no_mangle]
pub extern "C" fn dns_set_servers(servers_json: *const c_char) -> DnsResult {
    if servers_json.is_null() {
        return create_error_result("servers_json is null".to_string());
    }
    let json = unsafe { CStr::from_ptr(servers_json).to_string_lossy() };
    
    let servers: Vec<String> = match serde_json::from_str(&json) {
        Ok(s) => s,
        Err(e) => return create_error_result(e.to_string()),
    };

    let mut default_config = ResolverConfig::new();
    let mut domain_configs: HashMap<String, ResolverConfig> = HashMap::new();
    let mut system_domain_rules: Vec<String> = Vec::new(); // domains to use system DNS
    let mut has_default = false;
    let mut is_global_system = false;

    for s in servers {
        match parse_server_str(&s) {
            Ok(res) => {
                if res.is_system {
                    if let Some(domain) = res.domain {
                        system_domain_rules.push(domain);
                    } else {
                        has_default = true;
                        is_global_system = true;
                    }
                    continue;
                }

                let ns = hickory_resolver::config::NameServerConfig {
                    socket_addr: res.addr,
                    protocol: res.protocol,
                    tls_dns_name: res.tls_dns_name,
                    trust_negative_responses: false,
                    tls_config: None,
                    bind_addr: None
                };

                if let Some(domain) = res.domain {
                    let entry = domain_configs.entry(domain).or_insert_with(ResolverConfig::new);
                    entry.add_name_server(ns);
                } else {
                    default_config.add_name_server(ns);
                    has_default = true;
                }
            },
            Err(e) => return create_error_result(e),
        }
    }

    // Update Global Routing Table
    let mut routing_guard = ROUTING_TABLE.write().unwrap();
    routing_guard.clear();
    
    // 1. Add custom domain resolvers
    for (domain, cfg) in domain_configs {
        let resolver = RUNTIME.block_on(async {
            TokioAsyncResolver::tokio(cfg, ResolverOpts::default())
        });
        routing_guard.insert(domain, Arc::new(resolver));
    }

    // 2. Add system domain resolvers
    for domain in system_domain_rules {
        let resolver = RUNTIME.block_on(async {
            if let Ok((cfg, opts)) = hickory_resolver::system_conf::read_system_conf() {
                TokioAsyncResolver::tokio(cfg, opts)
            } else {
                TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default())
            }
        });
        routing_guard.insert(domain, Arc::new(resolver));
    }

    // Update Global Config
    let mut global_guard = GLOBAL_CONFIG.write().unwrap();
    if has_default {
        if is_global_system {
            *global_guard = None; // None means fallback to system DNS in get_resolver_by_id
        } else {
            *global_guard = Some(default_config);
        }
    } else {
        *global_guard = None;
    }

    create_success_result("{}".to_string())
}

 
#[no_mangle]
pub extern "C" fn dns_get_servers() -> DnsResult {
    let guard = GLOBAL_CONFIG.read().unwrap();
    let cfg = guard.clone().unwrap_or_else(ResolverConfig::default);
    let servers: Vec<String> = cfg.name_servers().iter().map(|ns| {
        let base = if let Some(tls_name) = &ns.tls_dns_name {
            format!("{}:{}", tls_name, ns.socket_addr.port())
        } else {
            ns.socket_addr.to_string()
        };

        match ns.protocol {
            Protocol::Tls => format!("tls://{}", base),
            Protocol::Https => format!("https://{}", base),
            Protocol::Quic => format!("quic://{}", base),
            Protocol::Tcp => format!("tcp://{}", base),
            Protocol::Udp => {
                if base.ends_with(":53") {
                    base[..base.len()-3].to_string()
                } else {
                    base
                }
            }
            _ => base,
        }
    }).collect();
    match serde_json::to_string(&servers) {
        Ok(s) => create_success_result(s),
        Err(e) => create_error_result(e.to_string())
    }
}

#[derive(Serialize, Clone)]
struct TlsaRecord {
    usage: u8,
    selector: u8,
    matching_type: u8,
    certificate_association_data: String, // Hex string or Base64? Node uses Buffer.
    // We can return a hex string, and JS side converts to ArrayBuffer.
}

#[no_mangle]
pub extern "C" fn dns_resolve_tlsa(id: i32, host: *const c_char) -> DnsResult {
    let host = unsafe { CStr::from_ptr(host).to_string_lossy().into_owned() };
    let host_copy = host.clone();
    resolve_helper(id, Some(&host_copy), "TLSA", |r| async move {
        let res = r.lookup(host, RecordType::TLSA).await?;
        let recs: Vec<TlsaRecord> = res.iter().filter_map(|r| {
             if let Some(t) = r.as_tlsa() {
                 Some(TlsaRecord {
                     usage: t.cert_usage().into(),
                     selector: t.selector().into(),
                     matching_type: t.matching().into(),
                     // Convert bytes to hex string for safe transport
                     certificate_association_data: hex::encode(t.cert_data()) 
                 })
             } else { None }
        }).collect();
        Ok(recs)
    })
}

#[no_mangle]
pub extern "C" fn dns_resolver_get_servers(id: i32) -> DnsResult {
    let store = RESOLVER_STORE.read().unwrap();
    if let Some((_resolver, config)) = store.get(&id) {
        let servers: Vec<String> = config.name_servers().iter().map(|ns| {
            let proto = match ns.protocol {
                Protocol::Udp => "udp",
                Protocol::Tcp => "tcp",
                Protocol::Tls => "tls",
                Protocol::Https => "https",
                Protocol::Quic => "quic",
                _ => "unknown",
            };
            if let Some(ref tls_name) = ns.tls_dns_name {
                format!("{}://{}#{}", proto, ns.socket_addr, tls_name)
            } else {
                format!("{}://{}", proto, ns.socket_addr)
            }
        }).collect();
        match serde_json::to_string(&servers) {
            Ok(s) => create_success_result(s),
            Err(e) => create_error_result(e.to_string())
        }
    } else {
        create_error_result(format!("Resolver with id {} not found", id))
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResolverConfigInput {
    servers: Option<Vec<String>>,
    timeout: Option<u64>, // milliseconds
    attempts: Option<usize>,
    #[allow(dead_code)] // Parsed for API compatibility but not applied by hickory-resolver
    max_timeout: Option<u64>,
    cache_size: Option<usize>,
    bind_v4: Option<String>,
    bind_v6: Option<String>,
}


#[no_mangle]
pub extern "C" fn dns_resolver_new(config_json: *const c_char) -> i32 {
    let json = unsafe { 
        if config_json.is_null() {
            return -1; // Error
        }
        CStr::from_ptr(config_json).to_string_lossy() 
    };
    
    // Parse config
    let (cfg, opts) = if json.is_empty() || json == "{}" {
        (ResolverConfig::default(), ResolverOpts::default())
    } else {
        // Try to parse as object
        match serde_json::from_str::<ResolverConfigInput>(&json) {
            Ok(input) => {
                // 1. Config (Servers)
                let bind_v4 = input.bind_v4.and_then(|s| s.parse::<std::net::IpAddr>().ok());
                let bind_v6 = input.bind_v6.and_then(|s| s.parse::<std::net::IpAddr>().ok());

                let config = if let Some(servers) = input.servers {
                    let mut c = ResolverConfig::new();
                    for s in servers {
                        if let Ok(res) = parse_server_str(&s) {
                            let bind_addr = if res.addr.is_ipv4() {
                                bind_v4.map(|ip| SocketAddr::new(ip, 0))
                            } else {
                                bind_v6.map(|ip| SocketAddr::new(ip, 0))
                            };

                            c.add_name_server(hickory_resolver::config::NameServerConfig {
                                socket_addr: res.addr, 
                                protocol: res.protocol, 
                                tls_dns_name: res.tls_dns_name, 
                                trust_negative_responses: false, 
                                tls_config: None, 
                                bind_addr
                            });
                        }
                    }
                    c
                } else {
                    ResolverConfig::default()
                };

                // 2. Options (Timeout, Attempts, max_timeout)
                let mut options = ResolverOpts::default();
                if let Some(t) = input.timeout {
                    if t > 0 {
                        options.timeout = std::time::Duration::from_millis(t);
                    }
                }
                if let Some(a) = input.attempts {
                    if a > 0 {
                        options.attempts = a;
                    }
                }
                if let Some(c) = input.cache_size {
                    options.cache_size = c;
                }
                // Note: max_timeout is not supported by hickory-resolver's ResolverOpts.
                // The field is parsed from config but not applied.

                (config, options)
            },
            Err(_) => {
                // Fallback: maybe it was just a list of servers? (Legacy support if any, mostly for backwards compat if we changed signature)
                // But we changed signature to expect object. 
                // Let's return error or default.
                return -1; 
            }
        }
    };
    
    let resolver = RUNTIME.block_on(async {
        TokioAsyncResolver::tokio(cfg.clone(), opts)
    });
    
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    let mut store = RESOLVER_STORE.write().unwrap();
    store.insert(id, (Arc::new(resolver), cfg));
    id
}

#[no_mangle]
pub extern "C" fn dns_resolver_delete(id: i32) {
    let mut store = RESOLVER_STORE.write().unwrap();
    store.remove(&id);
}

#[no_mangle]
pub extern "C" fn dns_resolver_set_local_address(id: i32, v4_ptr: *const c_char, v6_ptr: *const c_char) {
    let v4 = unsafe { CStr::from_ptr(v4_ptr).to_string_lossy() };
    let v6 = unsafe { CStr::from_ptr(v6_ptr).to_string_lossy() };
    
    let bind_v4 = v4.parse::<std::net::IpAddr>().ok();
    let bind_v6 = v6.parse::<std::net::IpAddr>().ok();

    let mut store = RESOLVER_STORE.write().unwrap();
    if let Some((_old_resolver, config)) = store.get_mut(&id) {
        // Update config
        let mut new_config = ResolverConfig::new();
        for ns in config.name_servers() {
            let mut new_ns = ns.clone();
            new_ns.bind_addr = if new_ns.socket_addr.is_ipv4() {
                bind_v4.map(|ip| SocketAddr::new(ip, 0))
            } else {
                bind_v6.map(|ip| SocketAddr::new(ip, 0))
            };
            new_config.add_name_server(new_ns);
        }
        
        // Re-create resolver (hickory-resolver is immutable)
        let new_resolver = RUNTIME.block_on(async {
            TokioAsyncResolver::tokio(new_config.clone(), ResolverOpts::default())
        });
        
        store.insert(id, (Arc::new(new_resolver), new_config));
    }
}

#[no_mangle]
pub extern "C" fn dns_resolver_clear_cache(id: i32) {
    if id == -1 {
        // Global/Default resolver cache clearing
        if let Some(resolver) = DEFAULT_RESOLVER.read().unwrap().as_ref() {
            resolver.clear_cache();
        }
    } else {
        let store = RESOLVER_STORE.read().unwrap();
        if let Some((resolver, _)) = store.get(&id) {
            resolver.clear_cache();
        }
    }
}

#[no_mangle]
pub extern "C" fn dns_clear_cache() {
    dns_resolver_clear_cache(-1);
}

#[no_mangle]
pub extern "C" fn dns_set_cache_size(size: usize) {
    let mut guard = DEFAULT_RESOLVER.write().unwrap();
    if let Some(_old_resolver) = guard.take() {
        tracing::info!("NitroDns: Global cache size updated to {}, re-creating default resolver on next query", size);
    }
    
    GLOBAL_OPTS_CACHE_SIZE.store(size, Ordering::SeqCst);
}

#[no_mangle]
pub extern "C" fn dns_set_cache_policy(policy: i32, stale_ttl: u32) {
    GLOBAL_CACHE_POLICY.store(policy, Ordering::SeqCst);
    GLOBAL_STALE_TTL.store(stale_ttl, Ordering::SeqCst);
    
    if policy == CachePolicy::Bypass as i32 {
        STALE_CACHE.write().unwrap().clear();
    }
}

// -------------------------------------------------------------------------
// Resolution Functions
// -------------------------------------------------------------------------
fn resolve_helper<F, T>(id: i32, hostname: Option<&str>, rrtype: &str, f: impl FnOnce(Arc<TokioAsyncResolver>) -> F) -> DnsResult 
where 
    F: std::future::Future<Output = Result<T, hickory_resolver::error::ResolveError>>,
    T: Serialize 
{
    let policy = GLOBAL_CACHE_POLICY.load(Ordering::SeqCst);
    let hostname_str = hostname.unwrap_or("");
    let cache_key = format!("{}:{}:{}", id, hostname_str, rrtype);

    if policy == CachePolicy::StaleWhileRevalidate as i32 {
        let now = Instant::now();
        let cache_hit = {
            let guard = STALE_CACHE.read().unwrap();
            guard.get(&cache_key).map(|(json, expiry, stale_expiry)| {
                (json.clone(), *expiry, *stale_expiry)
            })
        };

        if let Some((json, expiry, stale_expiry)) = cache_hit {
            if now < expiry {
                return create_success_result(json);
            } else if now < stale_expiry {
                return create_success_result(json);
            }
        }
    }

    // Get resolver
    let resolver = match get_resolver_by_id(id, hostname) {
        Ok(r) => r,
        Err(e) => return create_error_result(e),
    };
    
    // Log server info for debugging
    {
        let guard = GLOBAL_CONFIG.read().unwrap();
        let cfg_to_log: Option<ResolverConfig> = if let Some(cfg) = &*guard {
            Some(cfg.clone())
        } else {
            hickory_resolver::system_conf::read_system_conf().ok().map(|(cfg, _opts)| cfg)
        };
        
        if let Some(cfg) = cfg_to_log {
            tracing::info!("NitroDns: Query using {} DNS server(s):", cfg.name_servers().len());
            for ns in cfg.name_servers() {
                let proto = match ns.protocol {
                    Protocol::Udp => "udp",
                    Protocol::Tcp => "tcp",
                    Protocol::Tls => "tls",
                    Protocol::Https => "https",
                    Protocol::Quic => "quic",
                    _ => "unknown",
                };
                tracing::info!("  -> {}://{} (tls_name: {:?})", proto, ns.socket_addr, ns.tls_dns_name);
            }
        }
    }

    // Execute on RUNTIME
    let res = RUNTIME.block_on(async {
        f(resolver).await
    });

    match res {
        Ok(data) => {
            let json = serde_json::to_string(&data).unwrap_or_else(|_| "[]".to_string());
            
            if policy != CachePolicy::Bypass as i32 {
                let now = Instant::now();
                let ttl = 60; // Default fallback TTL
                let stale_grace = GLOBAL_STALE_TTL.load(Ordering::SeqCst) as u64;
                
                let mut guard = STALE_CACHE.write().unwrap();
                guard.insert(cache_key, (
                    json.clone(), 
                    now + Duration::from_secs(ttl),
                    now + Duration::from_secs(ttl + stale_grace)
                ));
            }
            create_success_result(json)
        }
        Err(e) => {
            if policy == CachePolicy::StaleIfError as i32 || policy == CachePolicy::StaleWhileRevalidate as i32 {
                let guard = STALE_CACHE.read().unwrap();
                if let Some((json, _, stale_expiry)) = guard.get(&cache_key) {
                    if Instant::now() < *stale_expiry {
                         return create_success_result(json.clone());
                    }
                }
            }
            create_error_result(e.to_string())
        }
    }
}

// -------------------------------------------------------------------------
// New Structs for Phase 1 & 2
// -------------------------------------------------------------------------
#[derive(Serialize)]
struct IpRecord {
    address: String,
    ttl: u32,
    family: u16,
}

#[derive(Serialize)]
struct LookupServiceResult {
    hostname: String,
    service: String,
}

// Implementations
#[no_mangle]
pub extern "C" fn dns_resolve_ipv4(id: i32, host: *const c_char) -> DnsResult {
    let host = unsafe { CStr::from_ptr(host).to_string_lossy().into_owned() };
    let host_copy = host.clone();
    resolve_helper(id, Some(&host_copy), "A", |r| async move {
        // Use lookup() instead of ipv4_lookup() to get access to Record (TTL)
        let res = r.lookup(host, RecordType::A).await?;
        let recs: Vec<IpRecord> = res.record_iter().filter_map(|rec| {
             rec.data().and_then(|d| d.as_a()).map(|a| IpRecord {
                 address: a.to_string(),
                 ttl: rec.ttl(),
                 family: 4
             })
        }).collect();
        Ok(recs)
    })
}

#[no_mangle]
pub extern "C" fn dns_resolve_ipv6(id: i32, host: *const c_char) -> DnsResult {
    let host = unsafe { CStr::from_ptr(host).to_string_lossy().into_owned() };
    let host_copy = host.clone();
    resolve_helper(id, Some(&host_copy), "AAAA", |r| async move {
        let res = r.lookup(host, RecordType::AAAA).await?;
        let recs: Vec<IpRecord> = res.record_iter().filter_map(|rec| {
            rec.data().and_then(|d| d.as_aaaa()).map(|aaaa| IpRecord {
                address: aaaa.to_string(),
                ttl: rec.ttl(),
                family: 6
            })
        }).collect();
        Ok(recs)
    })
}

#[derive(Serialize)]
struct MxRecord {
    exchange: String,
    priority: u16,
    ttl: u32,
}
#[no_mangle]
pub extern "C" fn dns_resolve_mx(id: i32, host: *const c_char) -> DnsResult {
    let host = unsafe { CStr::from_ptr(host).to_string_lossy().into_owned() };
    let host_copy = host.clone();
    resolve_helper(id, Some(&host_copy), "MX", |r| async move {
        let res = r.lookup(host, RecordType::MX).await?;
        let recs: Vec<MxRecord> = res.record_iter().filter_map(|rec| {
            rec.data().and_then(|d| d.as_mx()).map(|mx| MxRecord {
                exchange: mx.exchange().to_string(),
                priority: mx.preference(),
                ttl: rec.ttl()
            })
        }).collect();
        Ok(recs)
    })
}

#[derive(Serialize)]
struct TxtRecord {
    entries: Vec<String>,
    ttl: u32,
}
#[no_mangle]
pub extern "C" fn dns_resolve_txt(id: i32, host: *const c_char) -> DnsResult {
    let host = unsafe { CStr::from_ptr(host).to_string_lossy().into_owned() };
    let host_copy = host.clone();
    resolve_helper(id, Some(&host_copy), "TXT", |r| async move {
        let res = r.lookup(host, RecordType::TXT).await?;
        let recs: Vec<TxtRecord> = res.record_iter().filter_map(|rec| {
             rec.data().and_then(|d| d.as_txt()).map(|txt| TxtRecord {
                 entries: txt.txt_data().iter().map(|b| String::from_utf8_lossy(b).into_owned()).collect(),
                 ttl: rec.ttl()
             })
        }).collect();
        Ok(recs)
    })
}

#[no_mangle]
pub extern "C" fn dns_resolve_cname(id: i32, host: *const c_char) -> DnsResult {
    let host = unsafe { CStr::from_ptr(host).to_string_lossy().into_owned() };
    let host_copy = host.clone();
    resolve_helper(id, Some(&host_copy), "CNAME", |r| async move {
        let res = r.lookup(host, RecordType::CNAME).await?;
        let recs: Vec<String> = res.record_iter().filter_map(|rec| {
            rec.data().and_then(|d| d.as_cname()).map(|c| c.0.to_string())
        }).collect();
        Ok(recs)
    })
}
#[no_mangle]
pub extern "C" fn dns_resolve_ns(id: i32, host: *const c_char) -> DnsResult {
    let host = unsafe { CStr::from_ptr(host).to_string_lossy().into_owned() };
    let host_copy = host.clone();
    resolve_helper(id, Some(&host_copy), "NS", |r| async move {
        let res = r.lookup(host, RecordType::NS).await?;
        let recs: Vec<String> = res.record_iter().filter_map(|rec| {
            rec.data().and_then(|d| d.as_ns()).map(|c| c.0.to_string())
        }).collect();
        Ok(recs)
    })
}
#[no_mangle]
pub extern "C" fn dns_resolve_ptr(id: i32, host: *const c_char) -> DnsResult {
    let host = unsafe { CStr::from_ptr(host).to_string_lossy().into_owned() };
    let host_copy = host.clone();
    resolve_helper(id, Some(&host_copy), "PTR", |r| async move {
        let res = r.lookup(host, RecordType::PTR).await?;
        let recs: Vec<String> = res.record_iter().filter_map(|rec| {
            rec.data().and_then(|d| d.as_ptr()).map(|c| c.0.to_string())
        }).collect();
        Ok(recs)
    })
}

#[derive(Serialize, Clone)]
struct SoaRecord {
    nsname: String,
    hostmaster: String,
    serial: u32,
    refresh: i32,
    retry: i32,
    expire: i32,
    minttl: u32,
    ttl: u32,
}
#[no_mangle]
pub extern "C" fn dns_resolve_soa(id: i32, host: *const c_char) -> DnsResult {
    let host = unsafe { CStr::from_ptr(host).to_string_lossy().into_owned() };
    let host_copy = host.clone();
    resolve_helper(id, Some(&host_copy), "SOA", |r| async move {
        let res = r.lookup(host, RecordType::SOA).await?;
        let recs: Vec<SoaRecord> = res.record_iter().filter_map(|rec| {
            rec.data().and_then(|d| d.as_soa()).map(|soa| SoaRecord {
                nsname: soa.mname().to_string(),
                hostmaster: soa.rname().to_string(),
                serial: soa.serial(),
                refresh: soa.refresh(),
                retry: soa.retry(),
                expire: soa.expire(),
                minttl: soa.minimum(),
                ttl: rec.ttl()
            })
        }).collect();
        if recs.is_empty() {
             return Err(hickory_resolver::error::ResolveError::from("Has no SOA record"));
        }
        Ok(recs[0].clone())
    })
}

#[derive(Serialize)]
struct SrvRecord {
    priority: u16,
    weight: u16,
    port: u16,
    name: String,
    ttl: u32,
}
#[no_mangle]
pub extern "C" fn dns_resolve_srv(id: i32, host: *const c_char) -> DnsResult {
    let host = unsafe { CStr::from_ptr(host).to_string_lossy().into_owned() };
    let host_copy = host.clone();
    resolve_helper(id, Some(&host_copy), "SRV", |r| async move {
        let res = r.lookup(host, RecordType::SRV).await?;
        let recs: Vec<SrvRecord> = res.record_iter().filter_map(|rec| {
            rec.data().and_then(|d| d.as_srv()).map(|srv| SrvRecord {
                priority: srv.priority(),
                weight: srv.weight(),
                port: srv.port(),
                name: srv.target().to_string(),
                ttl: rec.ttl()
            })
        }).collect();
        Ok(recs)
    })
}

#[derive(Serialize)]
struct NaptrRecord {
    order: u16,
    preference: u16,
    flags: String,
    services: String,
    regexp: String,
    replacement: String,
    ttl: u32,
}
#[no_mangle]
pub extern "C" fn dns_resolve_naptr(id: i32, host: *const c_char) -> DnsResult {
    let host = unsafe { CStr::from_ptr(host).to_string_lossy().into_owned() };
    let host_copy = host.clone();
    resolve_helper(id, Some(&host_copy), "NAPTR", |r| async move {
        let res = r.lookup(host, RecordType::NAPTR).await?;
        let recs: Vec<NaptrRecord> = res.record_iter().filter_map(|rec| {
            rec.data().and_then(|d| d.as_naptr()).map(|naptr| NaptrRecord {
                order: naptr.order(),
                preference: naptr.preference(),
                flags: String::from_utf8_lossy(naptr.flags()).into_owned(),
                services: String::from_utf8_lossy(naptr.services()).into_owned(),
                regexp: String::from_utf8_lossy(naptr.regexp()).into_owned(),
                replacement: naptr.replacement().to_string(),
                ttl: rec.ttl()
            })
        }).collect();
        Ok(recs)
    })
}

#[derive(Serialize)]
struct CaaRecord {
    critical: u8,
    issue_option: String,
    value: String,
    ttl: u32,
}
#[no_mangle]
pub extern "C" fn dns_resolve_caa(id: i32, host: *const c_char) -> DnsResult {
    let host = unsafe { CStr::from_ptr(host).to_string_lossy().into_owned() };
    let host_copy = host.clone();
    resolve_helper(id, Some(&host_copy), "CAA", |r| async move {
        let res = r.lookup(host, RecordType::CAA).await?;
        let recs: Vec<CaaRecord> = res.record_iter().filter_map(|rec| {
            rec.data().and_then(|d| d.as_caa()).map(|caa| {
                let pair = caa.tag();
                let val = format!("{}", caa.value());
                let tag_str = match pair {
                    hickory_resolver::proto::rr::rdata::caa::Property::Issue => "issue",
                    hickory_resolver::proto::rr::rdata::caa::Property::IssueWild => "issuewild",
                    hickory_resolver::proto::rr::rdata::caa::Property::Iodef => "iodef",
                    _ => "unknown"
                };
                CaaRecord {
                    critical: if caa.issuer_critical() { 128 } else { 0 },
                    issue_option: tag_str.to_string(),
                    value: val,
                    ttl: rec.ttl()
                }
            })
        }).collect();
        Ok(recs)
    })
}

#[no_mangle]
pub extern "C" fn dns_reverse(id: i32, ip: *const c_char) -> DnsResult {
    let ip_str = unsafe { 
        if ip.is_null() { return create_error_result("Invalid IP".to_string()); }
        CStr::from_ptr(ip).to_string_lossy().into_owned() 
    };
    
    // Parse IP
    let ip_addr: std::net::IpAddr = match ip_str.parse() {
        Ok(ip) => ip,
        Err(e) => return create_error_result(format!("Invalid IP address: {}", e))
    };

    resolve_helper(id, None, "REVERSE", |r| async move {
        let res = r.reverse_lookup(ip_addr).await?;
        // reverse_lookup returns Lookup, which is iterator of QueryResult (names)
        let recs: Vec<String> = res.iter().map(|name| name.to_string()).collect();
        Ok(recs)
    })
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum AnyRecord {
    A { address: String, ttl: u32 },
    AAAA { address: String, ttl: u32 },
    MX { exchange: String, priority: u16, ttl: u32 },
    TXT { entries: Vec<String>, ttl: u32 },
    NS { value: String, ttl: u32 },
    CNAME { value: String, ttl: u32 },
    SOA { nsname: String, hostmaster: String, serial: u32, refresh: i32, retry: i32, expire: i32, minttl: u32, ttl: u32 },
    SRV { priority: u16, weight: u16, port: u16, name: String, ttl: u32 },
    CAA { critical: u8, issue_option: String, value: String, ttl: u32 }, // Simplification
    NAPTR { order: u16, preference: u16, flags: String, services: String, regexp: String, replacement: String, ttl: u32 },
    PTR { value: String, ttl: u32 },
    TLSA { usage: u8, selector: u8, matching_type: u8, certificate_association_data: String, ttl: u32 },
}

#[no_mangle]
pub extern "C" fn dns_resolve_any(id: i32, host: *const c_char) -> DnsResult {
    let host = unsafe { CStr::from_ptr(host).to_string_lossy().into_owned() };
    let host_owned = host.clone();
    resolve_helper(id, Some(&host), "ANY", |r| async move {
        let res = r.lookup(host_owned, RecordType::ANY).await?;
        let recs: Vec<AnyRecord> = res.record_iter().filter_map(|rec| {
            if let Some(a) = rec.data().and_then(|d| d.as_a()) {
                Some(AnyRecord::A { address: a.to_string(), ttl: rec.ttl() })
            } else if let Some(aaaa) = rec.data().and_then(|d| d.as_aaaa()) {
                Some(AnyRecord::AAAA { address: aaaa.to_string(), ttl: rec.ttl() })
            } else if let Some(mx) = rec.data().and_then(|d| d.as_mx()) {
                Some(AnyRecord::MX { exchange: mx.exchange().to_string(), priority: mx.preference(), ttl: rec.ttl() })
            } else if let Some(txt) = rec.data().and_then(|d| d.as_txt()) {
                Some(AnyRecord::TXT { 
                    entries: txt.txt_data().iter().map(|b| String::from_utf8_lossy(b).into_owned()).collect(),
                    ttl: rec.ttl() 
                })
            } else if let Some(ns) = rec.data().and_then(|d| d.as_ns()) {
                Some(AnyRecord::NS { value: ns.0.to_string(), ttl: rec.ttl() })
            } else if let Some(cname) = rec.data().and_then(|d| d.as_cname()) {
                Some(AnyRecord::CNAME { value: cname.0.to_string(), ttl: rec.ttl() })
            } else if let Some(soa) = rec.data().and_then(|d| d.as_soa()) {
                Some(AnyRecord::SOA {
                    nsname: soa.mname().to_string(),
                    hostmaster: soa.rname().to_string(),
                    serial: soa.serial(),
                    refresh: soa.refresh(),
                    retry: soa.retry(),
                    expire: soa.expire(),
                    minttl: soa.minimum(),
                    ttl: rec.ttl()
                })
            } else if let Some(srv) = rec.data().and_then(|d| d.as_srv()) {
                Some(AnyRecord::SRV {
                    priority: srv.priority(),
                    weight: srv.weight(),
                    port: srv.port(),
                    name: srv.target().to_string(),
                    ttl: rec.ttl()
                })
            } else if let Some(caa) = rec.data().and_then(|d| d.as_caa()) {
                // Determine tag/value string
                let pair = caa.tag(); // enum Property
                let val = format!("{}", caa.value());
                let tag_str = match pair {
                    hickory_resolver::proto::rr::rdata::caa::Property::Issue => "issue",
                    hickory_resolver::proto::rr::rdata::caa::Property::IssueWild => "issuewild",
                    hickory_resolver::proto::rr::rdata::caa::Property::Iodef => "iodef",
                    _ => "unknown"
                };
                Some(AnyRecord::CAA {
                    critical: if caa.issuer_critical() { 128 } else { 0 }, // Simplified
                    issue_option: tag_str.to_string(),
                    value: val,
                    ttl: rec.ttl()
                })
            } else if let Some(ptr) = rec.data().and_then(|d| d.as_ptr()) {
                Some(AnyRecord::PTR { value: ptr.0.to_string(), ttl: rec.ttl() })
            } else if let Some(naptr) = rec.data().and_then(|d| d.as_naptr()) {
                Some(AnyRecord::NAPTR {
                    order: naptr.order(),
                    preference: naptr.preference(),
                    flags: String::from_utf8_lossy(naptr.flags()).into_owned(),
                    services: String::from_utf8_lossy(naptr.services()).into_owned(),
                    regexp: String::from_utf8_lossy(naptr.regexp()).into_owned(),
                    replacement: naptr.replacement().to_string(),
                    ttl: rec.ttl()
                })
            } else if let Some(tlsa) = rec.data().and_then(|d| d.as_tlsa()) {
                Some(AnyRecord::TLSA {
                     usage: tlsa.cert_usage().into(),
                     selector: tlsa.selector().into(),
                     matching_type: tlsa.matching().into(),
                     certificate_association_data: hex::encode(tlsa.cert_data()),
                     ttl: rec.ttl()
                })
            } else {
                None
            }
        }).collect();
        Ok(recs)
    })
}

// System Lookup (getaddrinfo wrapper)
#[no_mangle]
pub extern "C" fn dns_lookup_sync(hostname: *const c_char, family: c_int) -> DnsResult {
    let host = unsafe {
        if hostname.is_null() { return create_error_result("Invalid hostname".to_string()); }
        CStr::from_ptr(hostname).to_string_lossy().into_owned()
    };
    tracing::info!("NitroDns: dns_lookup_sync called for {} family={}", host, family);
    
    let host_copy = host.clone();
    resolve_helper(-1, Some(&host_copy), "LOOKUP", |r| async move {
        let record_type = if family == 6 { RecordType::AAAA } else { RecordType::A };
        let res = r.lookup(host, record_type).await?;
        let filtered: Vec<String> = res.record_iter()
            .filter_map(|rec| {
                if family == 6 {
                    rec.data().and_then(|d| d.as_aaaa()).map(|aaaa| aaaa.to_string())
                } else {
                    rec.data().and_then(|d| d.as_a()).map(|a| a.to_string())
                }
            })
            .collect();
        Ok(filtered)
    })
}

// System Lookup Service (getnameinfo)
#[no_mangle]
pub extern "C" fn dns_lookup_service(ip: *const c_char, port: u16) -> DnsResult {
    let ip_str = unsafe { 
        if ip.is_null() { return create_error_result("Invalid IP".to_string()); }
        CStr::from_ptr(ip).to_string_lossy().into_owned() 
    };
    
    let addr: SocketAddr = match format!("{}:{}", ip_str, port).parse() {
        Ok(a) => a,
        Err(e) => return create_error_result(format!("Invalid IP/Port: {}", e))
    };

    match dns_lookup::getnameinfo(&addr, 0) {
        Ok((hostname, service)) => {
            let res = LookupServiceResult { hostname, service };
            match serde_json::to_string(&res) {
                 Ok(s) => create_success_result(s),
                 Err(e) => create_error_result(e.to_string())
            }
        },
        Err(e) => create_error_result(format!("{:?}", e))
    }
}

// --- JNI Support for Android Networking Integration ---
#[cfg(target_os = "android")]
#[allow(non_snake_case)]
pub mod android {
    use super::*;
    use jni::objects::{JClass, JObject, JString};
    use jni::sys::jobjectArray;
    use jni::JNIEnv;

    #[no_mangle]
    pub extern "system" fn Java_com_nitrodns_NitroDns_resolve(
        mut env: JNIEnv,
        _class: JClass,
        hostname: JString,
    ) -> jobjectArray {
        let hostname: String = match env.get_string(&hostname) {
            Ok(s) => s.into(),
            Err(_) => return env.new_object_array(0, "java/lang/String", JObject::default()).unwrap().into_raw(),
        };
        
        // Use the global runtime to block on the async resolver
        let ips: Vec<String> = RUNTIME.block_on(async {
            let resolver = get_resolver_by_id(-1, Some(&hostname)).unwrap();
            match resolver.lookup_ip(hostname).await {
                Ok(lookup) => lookup.iter().map(|ip| ip.to_string()).collect(),
                Err(_) => vec![],
            }
        });

        let array = match env.new_object_array(ips.len() as i32, "java/lang/String", JObject::default()) {
            Ok(a) => a,
            Err(_) => return ptr::null_mut(),
        };

        for (i, ip) in ips.into_iter().enumerate() {
            if let Ok(s) = env.new_string(ip) {
                let _ = env.set_object_array_element(&array, i as i32, s);
            }
        }
        array.into_raw()
    }
}

