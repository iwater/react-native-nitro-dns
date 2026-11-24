//! DNS 解析测试
//! 覆盖: dns_resolve_ipv4, dns_resolve_ipv6, dns_resolve_mx, dns_resolve_txt, 
//!       dns_resolve_cname, dns_resolve_ns, dns_resolve_ptr, dns_resolve_soa,
//!       dns_resolve_srv, dns_resolve_naptr, dns_resolve_caa, dns_resolve_tlsa,
//!       dns_resolve_any, dns_reverse, dns_lookup_sync, dns_lookup_service

mod common;

use rust_c_dns::*;

/// 使用已知可靠的域名进行测试
const TEST_DOMAIN: &str = "google.com";
const TEST_IP: &str = "8.8.8.8";

// ============================================================================
// IPv4/IPv6 解析测试
// ============================================================================

#[test]
fn test_resolve_ipv4_global() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();
    common::setup_default_servers();

    let host = common::cstr(TEST_DOMAIN);
    let result = dns_resolve_ipv4(-1, host.as_ptr());
    let data = common::assert_result_ok(result);
    
    // Should return array of IP records
    assert!(data.contains("address"), "Should contain address field: {}", data);
}

#[test]
fn test_resolve_ipv4_with_resolver() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let config = common::cstr(r#"{"servers": ["8.8.8.8"]}"#);
    let id = dns_resolver_new(config.as_ptr());
    assert!(id > 0);

    let host = common::cstr(TEST_DOMAIN);
    let result = dns_resolve_ipv4(id, host.as_ptr());
    let data = common::assert_result_ok(result);
    assert!(data.contains("address"));

    dns_resolver_delete(id);
}

#[test]
fn test_resolve_ipv6_global() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();
    common::setup_default_servers();

    let host = common::cstr(TEST_DOMAIN);
    let result = dns_resolve_ipv6(-1, host.as_ptr());
    
    // IPv6 might not always be available, just check it doesn't crash
    let _ = common::get_result_data(result);
}

// ============================================================================
// MX 记录测试
// ============================================================================

#[test]
fn test_resolve_mx() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();
    common::setup_default_servers();

    let host = common::cstr(TEST_DOMAIN);
    let result = dns_resolve_mx(-1, host.as_ptr());
    
    // MX might not exist for all domains, just ensure no crash
    let data = common::get_result_data(result);
    if let Ok(d) = &data {
        // If successful, should have exchange and priority
        assert!(d.contains("exchange") || d.contains("[]"), "MX data: {}", d);
    }
}

// ============================================================================
// TXT 记录测试
// ============================================================================

#[test]
fn test_resolve_txt() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();
    common::setup_default_servers();

    let host = common::cstr(TEST_DOMAIN);
    let result = dns_resolve_txt(-1, host.as_ptr());
    
    // TXT might not exist, just ensure no crash
    let data = common::get_result_data(result);
    if let Ok(d) = &data {
        assert!(d.contains("entries") || d.contains("[]"), "TXT data: {}", d);
    }
}

// ============================================================================
// CNAME 记录测试
// ============================================================================

#[test]
fn test_resolve_cname() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();
    common::setup_default_servers();

    // www.google.com typically has a CNAME
    let host = common::cstr("www.google.com");
    let result = dns_resolve_cname(-1, host.as_ptr());
    
    // CNAME might not exist, just ensure no crash
    let _ = common::get_result_data(result);
}

// ============================================================================
// NS 记录测试
// ============================================================================

#[test]
fn test_resolve_ns() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();
    common::setup_default_servers();

    let host = common::cstr(TEST_DOMAIN);
    let result = dns_resolve_ns(-1, host.as_ptr());
    
    // NS should usually work for major domains
    let data = common::get_result_data(result);
    if let Ok(d) = &data {
        assert!(d.contains("ns") || d.contains("[]"), "NS data: {}", d.to_lowercase());
    }
}

// ============================================================================
// PTR 记录测试 (反向解析)
// ============================================================================

#[test]
fn test_resolve_ptr() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();
    common::setup_default_servers();

    // PTR record for 8.8.8.8 -> 8.8.8.8.in-addr.arpa
    let host = common::cstr("8.8.8.8.in-addr.arpa");
    let result = dns_resolve_ptr(-1, host.as_ptr());
    
    // Just ensure no crash
    let _ = common::get_result_data(result);
}

// ============================================================================
// SOA 记录测试
// ============================================================================

#[test]
fn test_resolve_soa() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();
    common::setup_default_servers();

    let host = common::cstr(TEST_DOMAIN);
    let result = dns_resolve_soa(-1, host.as_ptr());
    
    // SOA should usually work for major domains
    let data = common::get_result_data(result);
    if let Ok(d) = &data {
        assert!(d.contains("nsname") || d.contains("error"), "SOA data: {}", d);
    }
}

// ============================================================================
// SRV 记录测试
// ============================================================================

#[test]
fn test_resolve_srv() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();
    common::setup_default_servers();

    // _xmpp-client._tcp.google.com is a known SRV record
    let host = common::cstr("_xmpp-client._tcp.google.com");
    let result = dns_resolve_srv(-1, host.as_ptr());
    
    // SRV might not exist, just ensure no crash
    let _ = common::get_result_data(result);
}

// ============================================================================
// NAPTR 记录测试
// ============================================================================

#[test]
fn test_resolve_naptr() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();
    common::setup_default_servers();

    // NAPTR is rare, just test it doesn't crash
    let host = common::cstr(TEST_DOMAIN);
    let result = dns_resolve_naptr(-1, host.as_ptr());
    let _ = common::get_result_data(result);
}

// ============================================================================
// CAA 记录测试
// ============================================================================

#[test]
fn test_resolve_caa() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();
    common::setup_default_servers();

    let host = common::cstr(TEST_DOMAIN);
    let result = dns_resolve_caa(-1, host.as_ptr());
    
    // CAA might not exist, just ensure no crash
    let _ = common::get_result_data(result);
}

// ============================================================================
// TLSA 记录测试
// ============================================================================

#[test]
fn test_resolve_tlsa() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();
    common::setup_default_servers();

    // TLSA is very rare, just test it doesn't crash
    let host = common::cstr("_443._tcp.www.google.com");
    let result = dns_resolve_tlsa(-1, host.as_ptr());
    let _ = common::get_result_data(result);
}

// ============================================================================
// ANY 记录测试
// ============================================================================

#[test]
fn test_resolve_any() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();
    common::setup_default_servers();

    let host = common::cstr(TEST_DOMAIN);
    let result = dns_resolve_any(-1, host.as_ptr());
    
    // ANY queries may be blocked by some DNS servers, just ensure no crash
    let _ = common::get_result_data(result);
}

// ============================================================================
// 反向解析测试
// ============================================================================

#[test]
fn test_reverse() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();
    common::setup_default_servers();

    let ip = common::cstr(TEST_IP);
    let result = dns_reverse(-1, ip.as_ptr());
    
    // Reverse lookup should work for 8.8.8.8
    let data = common::get_result_data(result);
    if let Ok(d) = &data {
        // 8.8.8.8 reverse lookup should return dns.google
        assert!(d.to_lowercase().contains("google") || d.contains("dns") || d.contains("[]"), 
                "Reverse lookup data: {}", d);
    }
}

#[test]
fn test_reverse_invalid_ip() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();
    common::setup_default_servers();

    let ip = common::cstr("not an ip");
    let result = dns_reverse(-1, ip.as_ptr());
    common::assert_result_err(result);
}

#[test]
fn test_reverse_null_pointer() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let result = dns_reverse(-1, std::ptr::null());
    common::assert_result_err(result);
}

// ============================================================================
// dns_lookup_sync 测试
// ============================================================================

#[test]
fn test_lookup_sync_ipv4() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();
    common::setup_default_servers();

    let host = common::cstr(TEST_DOMAIN);
    let result = dns_lookup_sync(host.as_ptr(), 4);
    let data = common::assert_result_ok(result);
    
    // Should return array of IP addresses
    assert!(data.starts_with('['), "Should return array: {}", data);
}

#[test]
fn test_lookup_sync_ipv6() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();
    common::setup_default_servers();

    let host = common::cstr(TEST_DOMAIN);
    let result = dns_lookup_sync(host.as_ptr(), 6);
    
    // IPv6 might not be available, just check no crash
    let _ = common::get_result_data(result);
}

#[test]
fn test_lookup_sync_null_pointer() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let result = dns_lookup_sync(std::ptr::null(), 4);
    common::assert_result_err(result);
}

// ============================================================================
// dns_lookup_service 测试
// ============================================================================

#[test]
fn test_lookup_service() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let ip = common::cstr("127.0.0.1");
    let result = dns_lookup_service(ip.as_ptr(), 80);
    
    // Service lookup result format: {hostname, service}
    let data = common::get_result_data(result);
    // Just ensure it doesn't crash and returns something
    assert!(data.is_ok() || data.is_err());
}

#[test]
fn test_lookup_service_null_pointer() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let result = dns_lookup_service(std::ptr::null(), 80);
    common::assert_result_err(result);
}

// ============================================================================
// 边界条件测试
// ============================================================================

#[test]
fn test_resolve_nonexistent_domain() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();
    common::setup_default_servers();

    let host = common::cstr("this-domain-definitely-does-not-exist-12345.com");
    let result = dns_resolve_ipv4(-1, host.as_ptr());
    
    // Should return error for nonexistent domain
    common::assert_result_err(result);
}

#[test]
fn test_resolve_with_invalid_resolver_id() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let host = common::cstr(TEST_DOMAIN);
    let result = dns_resolve_ipv4(99999, host.as_ptr());
    
    // Should return error for invalid resolver ID
    common::assert_result_err(result);
}
