//! FFI 安全测试
//! 覆盖: dns_free_result, null pointer handling, memory safety

mod common;

use rust_c_dns::*;
use std::ptr;

#[test]
fn test_free_result_with_success_data() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let servers = common::cstr(r#"["8.8.8.8"]"#);
    let result = dns_set_servers(servers.as_ptr());
    
    // Should not crash or leak memory
    dns_free_result(result);
}

#[test]
fn test_free_result_with_error_data() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let servers = common::cstr(r#"invalid json"#);
    let result = dns_set_servers(servers.as_ptr());
    
    // Should not crash or leak memory
    dns_free_result(result);
}

#[test]
fn test_free_result_with_null_pointers() {
    // Construct a DnsResult with null pointers
    let result = DnsResult {
        data: ptr::null_mut(),
        error: ptr::null_mut(),
    };
    
    // Should not crash
    dns_free_result(result);
}

#[test]
fn test_all_apis_with_null_host() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    // All resolution APIs should handle null host gracefully
    // (Note: Some may crash due to unsafe CStr::from_ptr, but we've added null checks)
    
    // dns_set_servers - should return error
    let result = dns_set_servers(ptr::null());
    let _ = common::get_result_data(result);
    
    // dns_resolver_new - should return -1
    let id = dns_resolver_new(ptr::null());
    assert_eq!(id, -1);
    
    // dns_reverse - should return error
    let result = dns_reverse(-1, ptr::null());
    let _ = common::get_result_data(result);
    
    // dns_lookup_sync - should return error
    let result = dns_lookup_sync(ptr::null(), 4);
    let _ = common::get_result_data(result);
    
    // dns_lookup_service - should return error
    let result = dns_lookup_service(ptr::null(), 80);
    let _ = common::get_result_data(result);
}

#[test]
fn test_resolver_lifecycle_stability() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    // Create and delete many resolvers to test for leaks/stability
    for _ in 0..10 {
        let config = common::cstr(r#"{"servers": ["8.8.8.8"]}"#);
        let id = dns_resolver_new(config.as_ptr());
        assert!(id > 0);
        dns_resolver_delete(id);
    }
}

#[test]
fn test_concurrent_resolver_creation() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    // Create multiple resolvers concurrently (simulated sequentially here)
    let mut ids = Vec::new();
    for i in 0..5 {
        let config = common::cstr(&format!(r#"{{"servers": ["8.8.8.{}"], "timeout": {}}}"#, i + 1, 1000 + i * 100));
        let id = dns_resolver_new(config.as_ptr());
        assert!(id > 0);
        ids.push(id);
    }

    // All IDs should be unique
    let unique_count = {
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        sorted.len()
    };
    assert_eq!(unique_count, ids.len(), "All resolver IDs should be unique");

    // Delete all resolvers
    for id in ids {
        dns_resolver_delete(id);
    }
}

#[test]
fn test_set_servers_repeatedly() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    // Set servers many times to test for memory leaks
    for i in 0..20 {
        let servers = common::cstr(&format!(r#"["8.8.8.{}"]"#, (i % 254) + 1));
        let result = dns_set_servers(servers.as_ptr());
        let _ = common::get_result_data(result);
    }
}

#[test]
fn test_get_servers_repeatedly() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let servers = common::cstr(r#"["8.8.8.8"]"#);
    let _ = common::get_result_data(dns_set_servers(servers.as_ptr()));

    // Get servers many times
    for _ in 0..10 {
        let result = dns_get_servers();
        let _ = common::get_result_data(result);
    }
}

#[test]
fn test_special_characters_in_hostname() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();
    common::setup_default_servers();

    // Test with various special characters
    let hosts = [
        "example.com",
        "sub.example.com",
        "sub-domain.example.com",
        "123.example.com",
        // Note: punycode domains should work
    ];

    for host in &hosts {
        let h = common::cstr(host);
        let result = dns_resolve_ipv4(-1, h.as_ptr());
        let _ = common::get_result_data(result); // Just ensure no crash
    }
}

#[test]
fn test_very_long_hostname() {
    let _lock = common::TEST_LOCK.lock().unwrap();
    common::init_test_env();
    common::setup_default_servers();

    // Create a very long hostname (but still within DNS limits)
    let long_subdomain = "a".repeat(63);
    let host = format!("{}.{}.{}.example.com", long_subdomain, long_subdomain, "b".repeat(63));
    
    let h = common::cstr(&host);
    let result = dns_resolve_ipv4(-1, h.as_ptr());
    
    // Should fail but not crash
    let _ = common::get_result_data(result);
}

#[test]
fn test_unicode_hostname() {
    let _lock = common::TEST_LOCK.lock().unwrap();
    common::init_test_env();
    common::setup_default_servers();

    // Unicode hostname (will be converted to punycode internally)
    let host = common::cstr("münchen.de");
    let result = dns_resolve_ipv4(-1, host.as_ptr());
    
    // Should handle gracefully
    let _ = common::get_result_data(result);
}
