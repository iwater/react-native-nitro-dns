//! DNS 配置管理测试
//! 覆盖: dns_set_servers, dns_get_servers

mod common;

use rust_c_dns::*;

#[test]
fn test_set_servers_basic() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let servers = common::cstr(r#"["8.8.8.8", "1.1.1.1"]"#);
    let result = dns_set_servers(servers.as_ptr());
    let data = common::assert_result_ok(result);
    assert_eq!(data, "{}");
}

#[test]
fn test_set_servers_with_protocols() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    // UDP (default)
    let servers = common::cstr(r#"["udp://8.8.8.8"]"#);
    let result = dns_set_servers(servers.as_ptr());
    common::assert_result_ok(result);

    // TCP
    let servers = common::cstr(r#"["tcp://8.8.8.8"]"#);
    let result = dns_set_servers(servers.as_ptr());
    common::assert_result_ok(result);

    // DoT (DNS over TLS)
    let servers = common::cstr(r#"["tls://8.8.8.8:853"]"#);
    let result = dns_set_servers(servers.as_ptr());
    common::assert_result_ok(result);

    // DoH (DNS over HTTPS)
    let servers = common::cstr(r#"["https://8.8.8.8/dns-query"]"#);
    let result = dns_set_servers(servers.as_ptr());
    common::assert_result_ok(result);
}

#[test]
fn test_set_servers_with_domain_routing() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    // dnsmasq-style domain routing
    let servers = common::cstr(r#"["/google.com/8.8.8.8", "/cloudflare.com/1.1.1.1", "9.9.9.9"]"#);
    let result = dns_set_servers(servers.as_ptr());
    common::assert_result_ok(result);
}

#[test]
fn test_set_servers_with_system_keyword() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    // System DNS routing
    let servers = common::cstr(r#"["/local/system", "8.8.8.8"]"#);
    let result = dns_set_servers(servers.as_ptr());
    common::assert_result_ok(result);
}

#[test]
fn test_set_servers_empty() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let servers = common::cstr(r#"[]"#);
    let result = dns_set_servers(servers.as_ptr());
    common::assert_result_ok(result);
}

#[test]
fn test_set_servers_invalid_json() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let servers = common::cstr(r#"invalid json"#);
    let result = dns_set_servers(servers.as_ptr());
    common::assert_result_err(result);
}

#[test]
fn test_set_servers_null_pointer() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    let result = dns_set_servers(std::ptr::null());
    let err = common::assert_result_err(result);
    assert!(err.contains("null"), "Error should mention null: {}", err);
}

#[test]
fn test_get_servers() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    // Set servers first
    let servers = common::cstr(r#"["8.8.8.8"]"#);
    let result = dns_set_servers(servers.as_ptr());
    common::assert_result_ok(result);

    // Get servers
    let result = dns_get_servers();
    let data = common::assert_result_ok(result);
    
    // Should contain 8.8.8.8
    assert!(data.contains("8.8.8.8"), "Should contain 8.8.8.8: {}", data);
}

#[test]
fn test_set_verbose() {
    // dns_set_verbose should not crash
    dns_set_verbose(true);
    dns_set_verbose(false);
}
