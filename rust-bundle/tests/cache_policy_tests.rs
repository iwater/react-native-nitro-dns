mod common;

use rust_c_dns::*;

#[test]
fn test_set_cache_size() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();
    common::setup_default_servers();

    // Set a large cache size
    dns_set_cache_size(1000);
    
    // Initial resolve to ensure default resolver is created
    let host = common::cstr("google.com");
    let res1 = dns_resolve_ipv4(-1, host.as_ptr());
    common::assert_result_ok(res1);

    // Set a different cache size, should clear the default resolver
    dns_set_cache_size(10);
    
    let res2 = dns_resolve_ipv4(-1, host.as_ptr());
    common::assert_result_ok(res2);
}

#[test]
fn test_cache_policy_bypass() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();
    common::setup_default_servers();

    // Set policy to Bypass (1)
    dns_set_cache_policy(1, 3600);
    
    let host = common::cstr("google.com");
    let res = dns_resolve_ipv4(-1, host.as_ptr());
    common::assert_result_ok(res);
}

#[test]
fn test_cache_policy_swr() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();
    common::setup_default_servers();

    // Set policy to StaleWhileRevalidate (2) with 1 hour stale grace
    dns_set_cache_policy(2, 3600);
    
    let host = common::cstr("google.com");
    let res = dns_resolve_ipv4(-1, host.as_ptr());
    common::assert_result_ok(res);
}

#[test]
fn test_cache_policy_sie() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();
    common::setup_default_servers();

    // Set policy to StaleIfError (3)
    dns_set_cache_policy(3, 3600);
    
    let host = common::cstr("google.com");
    let res = dns_resolve_ipv4(-1, host.as_ptr());
    common::assert_result_ok(res);
}

#[test]
fn test_resolver_specific_cache_size() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();

    // Create a resolver with specific cache size
    let config = common::cstr(r#"{"servers": ["8.8.8.8"], "cache_size": 100}"#);
    let id = dns_resolver_new(config.as_ptr());
    assert!(id > 0);

    let host = common::cstr("google.com");
    let result = dns_resolve_ipv4(id, host.as_ptr());
    common::assert_result_ok(result);

    // Clear cache for this resolver
    dns_resolver_clear_cache(id);
    
    let result2 = dns_resolve_ipv4(id, host.as_ptr());
    common::assert_result_ok(result2);

    dns_resolver_delete(id);
}

#[test]
fn test_global_clear_cache() {
    let _lock = common::acquire_test_lock();
    common::init_test_env();
    common::setup_default_servers();

    let host = common::cstr("google.com");
    let res1 = dns_resolve_ipv4(-1, host.as_ptr());
    common::assert_result_ok(res1);

    // Clear global cache
    dns_clear_cache();
    
    let res2 = dns_resolve_ipv4(-1, host.as_ptr());
    common::assert_result_ok(res2);
}
