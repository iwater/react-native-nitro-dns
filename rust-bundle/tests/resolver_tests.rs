//! DNS Resolver 生命周期测试
//! 覆盖: dns_resolver_new, dns_resolver_delete, dns_resolver_get_servers

mod common;

use rust_c_dns::*;

#[test]
fn test_resolver_new_default() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    // Create resolver with empty config
    let config = common::cstr(r#"{}"#);
    let id = dns_resolver_new(config.as_ptr());
    assert!(id > 0, "Resolver ID should be positive");

    // Cleanup
    dns_resolver_delete(id);
}

#[test]
fn test_resolver_new_with_servers() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let config = common::cstr(r#"{"servers": ["8.8.8.8", "1.1.1.1"]}"#);
    let id = dns_resolver_new(config.as_ptr());
    assert!(id > 0, "Resolver ID should be positive");

    dns_resolver_delete(id);
}

#[test]
fn test_resolver_new_with_timeout() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let config = common::cstr(r#"{"servers": ["8.8.8.8"], "timeout": 5000}"#);
    let id = dns_resolver_new(config.as_ptr());
    assert!(id > 0, "Resolver ID should be positive");

    dns_resolver_delete(id);
}

#[test]
fn test_resolver_new_with_attempts() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let config = common::cstr(r#"{"servers": ["8.8.8.8"], "attempts": 3}"#);
    let id = dns_resolver_new(config.as_ptr());
    assert!(id > 0, "Resolver ID should be positive");

    dns_resolver_delete(id);
}

#[test]
fn test_resolver_new_full_config() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let config = common::cstr(r#"{
        "servers": ["8.8.8.8", "tls://1.1.1.1:853"],
        "timeout": 3000,
        "attempts": 2
    }"#);
    let id = dns_resolver_new(config.as_ptr());
    assert!(id > 0, "Resolver ID should be positive");

    dns_resolver_delete(id);
}

#[test]
fn test_resolver_new_null_pointer() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let id = dns_resolver_new(std::ptr::null());
    assert_eq!(id, -1, "Should return -1 for null pointer");
}

#[test]
fn test_resolver_new_invalid_json() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let config = common::cstr(r#"invalid json"#);
    let id = dns_resolver_new(config.as_ptr());
    assert_eq!(id, -1, "Should return -1 for invalid JSON");
}

#[test]
fn test_resolver_delete_nonexistent() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    // Should not crash when deleting nonexistent resolver
    dns_resolver_delete(99999);
}

#[test]
fn test_resolver_get_servers() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let config = common::cstr(r#"{"servers": ["8.8.8.8"]}"#);
    let id = dns_resolver_new(config.as_ptr());
    assert!(id > 0);

    let result = dns_resolver_get_servers(id);
    let data = common::get_result_data(result).expect("Should return data");
    
    // Parse the returned JSON array
    let servers: Vec<String> = serde_json::from_str(&data).expect("Should be valid JSON array");
    assert_eq!(servers.len(), 1);
    assert!(servers[0].contains("8.8.8.8"));

    dns_resolver_delete(id);
}

#[test]
fn test_resolver_get_servers_multiple() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let config = common::cstr(r#"{"servers": ["8.8.8.8", "1.1.1.1", "9.9.9.9"]}"#);
    let id = dns_resolver_new(config.as_ptr());
    assert!(id > 0);

    let result = dns_resolver_get_servers(id);
    let data = common::get_result_data(result).expect("Should return data");
    
    let servers: Vec<String> = serde_json::from_str(&data).expect("Should be valid JSON array");
    assert_eq!(servers.len(), 3);
    assert!(servers.iter().any(|s| s.contains("8.8.8.8")));
    assert!(servers.iter().any(|s| s.contains("1.1.1.1")));
    assert!(servers.iter().any(|s| s.contains("9.9.9.9")));

    dns_resolver_delete(id);
}

#[test]
fn test_resolver_get_servers_with_protocols() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let config = common::cstr(r#"{"servers": ["udp://8.8.8.8", "tls://1.1.1.1:853", "https://dns.google/dns-query"]}"#);
    let id = dns_resolver_new(config.as_ptr());
    assert!(id > 0);

    let result = dns_resolver_get_servers(id);
    let data = common::get_result_data(result).expect("Should return data");
    
    let servers: Vec<String> = serde_json::from_str(&data).expect("Should be valid JSON array");
    assert_eq!(servers.len(), 3);
    
    // Check protocols are preserved
    assert!(servers.iter().any(|s| s.starts_with("udp://")));
    assert!(servers.iter().any(|s| s.starts_with("tls://")));
    assert!(servers.iter().any(|s| s.starts_with("https://")));

    dns_resolver_delete(id);
}

#[test]
fn test_resolver_get_servers_nonexistent() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let result = dns_resolver_get_servers(99999);
    // Should return error for nonexistent resolver
    assert!(!result.error.is_null());
    dns_free_result(result);
}


#[test]
fn test_resolver_multiple_instances() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let config1 = common::cstr(r#"{"servers": ["8.8.8.8"]}"#);
    let config2 = common::cstr(r#"{"servers": ["1.1.1.1"]}"#);
    let config3 = common::cstr(r#"{"servers": ["9.9.9.9"]}"#);

    let id1 = dns_resolver_new(config1.as_ptr());
    let id2 = dns_resolver_new(config2.as_ptr());
    let id3 = dns_resolver_new(config3.as_ptr());

    assert!(id1 > 0 && id2 > 0 && id3 > 0);
    assert!(id1 != id2 && id2 != id3 && id1 != id3, "IDs should be unique");

    dns_resolver_delete(id1);
    dns_resolver_delete(id2);
    dns_resolver_delete(id3);
}

#[test]
fn test_resolver_new_with_max_timeout() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    // Note: maxTimeout is parsed for API compatibility but not applied by hickory-resolver.
    // This test verifies it doesn't cause parsing errors.
    let config = common::cstr(r#"{"servers": ["8.8.8.8"], "maxTimeout": 10000}"#);
    let id = dns_resolver_new(config.as_ptr());
    assert!(id > 0, "Resolver ID should be positive");

    dns_resolver_delete(id);
}

#[test]
fn test_resolver_set_local_address() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let config = common::cstr(r#"{"servers": ["8.8.8.8"]}"#);
    let id = dns_resolver_new(config.as_ptr());
    assert!(id > 0);

    // Test setting local address
    let v4 = common::cstr("127.0.0.1");
    let v6 = common::cstr("::1");
    dns_resolver_set_local_address(id, v4.as_ptr(), v6.as_ptr());
    
    // Check it still works (doesn't crash)
    let result = dns_resolver_get_servers(id);
    let data = common::get_result_data(result).expect("Should return data");
    assert!(data.contains("8.8.8.8"));

    dns_resolver_delete(id);
}

#[test]
fn test_resolver_new_with_bind_addresses() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let config = common::cstr(r#"{
        "servers": ["8.8.8.8"],
        "bindV4": "127.0.0.1",
        "bindV6": "::1"
    }"#);
    let id = dns_resolver_new(config.as_ptr());
    assert!(id > 0, "Resolver ID should be positive");

    dns_resolver_delete(id);
}
