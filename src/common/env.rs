//! 公共环境变量展开
//!
//! v1.3 起供 config.rs 和 llm/preset_loader.rs 共享。
//! 展开 ${ENV_VAR} 形式的字符串——未设置的环境变量留原样（不报错）。

/// 展开 input 中所有 ${ENV_VAR} 占位符
/// - 设置了 → 替换为值
/// - 未设置 → 留原样 `${VAR_NAME}`（**不**报错，调用方决定如何处理）
pub fn expand_env_vars(input: &str) -> Result<String, crate::common::error::EflowError> {
    let re = regex_lite::Regex::new(r"\$\{(\w+)\}").map_err(|e| {
        crate::common::error::EflowError::Internal(format!("Failed to compile regex: {}", e))
    })?;
    let result = re
        .replace_all(input, |caps: &regex_lite::Captures| {
            let var_name = &caps[1];
            std::env::var(var_name).unwrap_or_else(|_| format!("${{{var_name}}}"))
        })
        .to_string();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_env_vars_substitutes_known_var() {
        // SAFETY: 单线程测试中设置环境变量不会与其他测试产生数据竞争
        unsafe {
            std::env::set_var("EFLOW_TEST_VAR_EXPAND", "hello");
        }
        let out = expand_env_vars("key=${EFLOW_TEST_VAR_EXPAND}").unwrap();
        assert_eq!(out, "key=hello");
    }

    #[test]
    fn expand_env_vars_leaves_unknown_var_intact() {
        let out = expand_env_vars("key=${EFLOW_NONEXISTENT_XYZ_999}").unwrap();
        assert_eq!(out, "key=${EFLOW_NONEXISTENT_XYZ_999}");
    }

    #[test]
    fn expand_env_vars_handles_multiple_vars() {
        unsafe {
            std::env::set_var("EFLOW_TEST_A", "alpha");
            std::env::set_var("EFLOW_TEST_B", "beta");
        }
        let out = expand_env_vars("${EFLOW_TEST_A}-${EFLOW_TEST_B}").unwrap();
        assert_eq!(out, "alpha-beta");
    }
}
