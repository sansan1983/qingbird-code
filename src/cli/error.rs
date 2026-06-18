//! exit code 转换 + Ctrl+C 处理
//!
//! 4 档退出码（契约冻结 v1.3.0 起 — spec B2 ADR-0017）：
//! - 0 = ok
//! - 1 = 用户错误（参数非法、文件不存在、KEY 无效等）
//! - 2 = 系统错误（网络失败、文件 IO 错误、内部错误）
//! - 130 = Ctrl+C
//!
//! 设计原则（spec B2 §6 风险表）：GUI 拿到非 0 exit code 时能区分
//! "重试就有用"（系统错误 2）vs "重试也白搭"（用户错误 1）。

use std::process::ExitCode;

use crate::common::error::EflowError;

/// EflowError → exit code 转换
///
/// 实际 EflowError 变体（v1.3.2 收尾时共 14 个）：
///   Config / LlmAuthFailed / ProfileNotFound / SkillNotFound /
///   PermissionDenied / RiskEscalated / TaskCancelled
///   → 用户错误（exit code 1）—— GUI 重试也没用
///
///   LlmProvider / RateLimited / Io / Memory / Internal / Tool
///   → 系统错误（exit code 2）—— GUI 可考虑重试
pub fn exit_code(err: &EflowError) -> ExitCode {
    match err {
        // 用户错误
        EflowError::Config(_)
        | EflowError::LlmAuthFailed(_)
        | EflowError::ProfileNotFound(_)
        | EflowError::SkillNotFound(_)
        | EflowError::PermissionDenied(_)
        | EflowError::RiskEscalated { .. }
        | EflowError::TaskCancelled(_)
        | EflowError::Serialization(_) => ExitCode::from(1),

        // 系统错误
        EflowError::LlmProvider(_)
        | EflowError::RateLimited(_)
        | EflowError::Io(_)
        | EflowError::Memory(_)
        | EflowError::Internal(_)
        | EflowError::Tool(_) => ExitCode::from(2),
    }
}

/// Ctrl+C 退出码 130（Unix 习惯 = 128 + SIGINT(2)）
pub fn handle_sigint() -> ExitCode {
    ExitCode::from(130)
}

#[cfg(test)]
mod tests {
    use super::*;

    // v1.3.2 T3: exit code 转换单元测试
    // 覆盖 EflowError 每个变体到 0/1/2 的映射 + Ctrl+C 130

    #[test]
    fn config_error_maps_to_user_error_1() {
        let err = EflowError::Config("bad config".into());
        assert_eq!(exit_code(&err), ExitCode::from(1));
    }

    #[test]
    fn auth_failed_maps_to_user_error_1() {
        let err = EflowError::LlmAuthFailed("anthropic".into());
        assert_eq!(exit_code(&err), ExitCode::from(1));
    }

    #[test]
    fn profile_not_found_maps_to_user_error_1() {
        let err = EflowError::ProfileNotFound("dev".into());
        assert_eq!(exit_code(&err), ExitCode::from(1));
    }

    #[test]
    fn skill_not_found_maps_to_user_error_1() {
        let err = EflowError::SkillNotFound("x".into());
        assert_eq!(exit_code(&err), ExitCode::from(1));
    }

    #[test]
    fn permission_denied_maps_to_user_error_1() {
        let err = EflowError::PermissionDenied("x".into());
        assert_eq!(exit_code(&err), ExitCode::from(1));
    }

    #[test]
    fn risk_escalated_maps_to_user_error_1() {
        let err = EflowError::RiskEscalated {
            task_id: "x".into(),
            reason: "y".into(),
        };
        assert_eq!(exit_code(&err), ExitCode::from(1));
    }

    #[test]
    fn task_cancelled_maps_to_user_error_1() {
        let err = EflowError::TaskCancelled("x".into());
        assert_eq!(exit_code(&err), ExitCode::from(1));
    }

    #[test]
    fn serialization_error_maps_to_user_error_1() {
        let err = EflowError::Serialization("x".into());
        assert_eq!(exit_code(&err), ExitCode::from(1));
    }

    #[test]
    fn provider_error_maps_to_system_error_2() {
        let err = EflowError::LlmProvider("network down".into());
        assert_eq!(exit_code(&err), ExitCode::from(2));
    }

    #[test]
    fn rate_limited_maps_to_system_error_2() {
        let err = EflowError::RateLimited("anthropic".into());
        assert_eq!(exit_code(&err), ExitCode::from(2));
    }

    #[test]
    fn internal_error_maps_to_system_error_2() {
        let err = EflowError::Internal("oops".into());
        assert_eq!(exit_code(&err), ExitCode::from(2));
    }

    #[test]
    fn io_error_maps_to_system_error_2() {
        let err = EflowError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "x"));
        assert_eq!(exit_code(&err), ExitCode::from(2));
    }

    #[test]
    fn memory_error_maps_to_system_error_2() {
        let err = EflowError::Memory("x".into());
        assert_eq!(exit_code(&err), ExitCode::from(2));
    }

    #[test]
    fn tool_error_maps_to_system_error_2() {
        let err = EflowError::Tool("x".into());
        assert_eq!(exit_code(&err), ExitCode::from(2));
    }

    #[test]
    fn handle_sigint_returns_130() {
        assert_eq!(handle_sigint(), ExitCode::from(130));
    }
}
