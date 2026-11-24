//! 共享测试工具模块
//! 提供 DNS 测试辅助函数

use std::ffi::{CStr, CString};
use std::sync::{Mutex, MutexGuard, Once};
use lazy_static::lazy_static;

// Re-export rust_c_dns from the crate being tested
pub use rust_c_dns::DnsResult;

lazy_static! {
    /// 全局测试锁 - 确保测试顺序执行（因为 DNS 库有全局状态）
    pub static ref TEST_LOCK: Mutex<()> = Mutex::new(());
}

static INIT: Once = Once::new();

/// 初始化测试环境
pub fn init_test_env() {
    INIT.call_once(|| {
        let _ = env_logger::builder().is_test(true).try_init();
    });
}

/// 获取测试锁（处理锁中毒情况）
pub fn acquire_test_lock() -> MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap_or_else(|poisoned| {
        // 恢复锁中毒状态，继续测试
        poisoned.into_inner()
    })
}

/// 从 DnsResult 提取成功数据
pub fn get_result_data(result: DnsResult) -> Result<String, String> {
    let data = if result.data.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(result.data).to_string_lossy().into_owned() })
    };

    let error = if result.error.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(result.error).to_string_lossy().into_owned() })
    };

    // Free the result
    rust_c_dns::dns_free_result(result);

    match (data, error) {
        (Some(d), None) => Ok(d),
        (None, Some(e)) => Err(e),
        (Some(d), Some(_)) => Ok(d), // Data takes precedence
        (None, None) => Err("Empty result".to_string()),
    }
}

/// 断言 DnsResult 成功
#[allow(dead_code)]
pub fn assert_result_ok(result: DnsResult) -> String {
    let data = get_result_data(result);
    assert!(data.is_ok(), "Expected success, got error: {:?}", data.err());
    data.unwrap()
}

/// 断言 DnsResult 失败
#[allow(dead_code)]
pub fn assert_result_err(result: DnsResult) -> String {
    let data = get_result_data(result);
    assert!(data.is_err(), "Expected error, got success: {:?}", data.ok());
    data.unwrap_err()
}

/// 创建 C 字符串
pub fn cstr(s: &str) -> CString {
    CString::new(s).expect("CString::new failed")
}

/// 设置默认 DNS 服务器 (Google DNS)
#[allow(dead_code)]
pub fn setup_default_servers() {

    let servers = cstr(r#"["8.8.8.8"]"#);
    let result = rust_c_dns::dns_set_servers(servers.as_ptr());
    let _ = get_result_data(result); // Free the result
}

/// 清理 DNS 服务器配置
#[allow(dead_code)]
pub fn cleanup_servers() {
    let servers = cstr(r#"[]"#);
    let result = rust_c_dns::dns_set_servers(servers.as_ptr());
    let _ = get_result_data(result);
}
