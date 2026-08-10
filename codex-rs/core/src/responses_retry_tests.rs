use super::ResponsesStreamRequest;
use super::log_retry;
use crate::session::tests::make_session_and_context;
use codex_config::config_toml::StreamRetryRule;
use codex_protocol::error::CodexErr;
use std::time::Duration;
use tracing_test::internal::MockWriter;

fn rule(error_codes: &[&str], message_contains: &[&str], max_retries: u64) -> StreamRetryRule {
    StreamRetryRule {
        error_codes: error_codes.iter().map(ToString::to_string).collect(),
        message_contains: message_contains.iter().map(ToString::to_string).collect(),
        max_retries,
    }
}

#[test]
fn server_overloaded_uses_default_retry_limit() {
    assert_eq!(
        super::retry_limit_for_response_stream_error(&[], 5, &CodexErr::ServerOverloaded),
        Some(5)
    );
}

#[test]
fn structured_error_code_overrides_retry_limit() {
    let rules = [rule(&["server_overloaded"], &[], 8)];
    assert_eq!(
        super::retry_limit_for_response_stream_error(&rules, 5, &CodexErr::ServerOverloaded),
        Some(8)
    );
}

#[test]
fn message_rule_matches_case_insensitively() {
    let rules = [rule(&[], &["SELECTED MODEL IS AT CAPACITY"], 8)];
    assert_eq!(
        super::retry_limit_for_response_stream_error(&rules, 5, &CodexErr::ServerOverloaded),
        Some(8)
    );
}

#[test]
fn zero_retry_rule_disables_overload_retry() {
    let rules = [rule(&["server_overloaded"], &[], 0)];
    assert_eq!(
        super::retry_limit_for_response_stream_error(&rules, 5, &CodexErr::ServerOverloaded),
        Some(0)
    );
}

#[test]
fn terminal_errors_cannot_be_enabled_by_message_rule() {
    let rules = [rule(&[], &["quota exceeded"], 8)];
    assert_eq!(
        super::retry_limit_for_response_stream_error(&rules, 5, &CodexErr::QuotaExceeded),
        None
    );
}

#[test]
fn first_matching_rule_wins() {
    let rules = [
        rule(&["server_overloaded"], &[], 2),
        rule(&[], &["capacity"], 9),
    ];
    assert_eq!(
        super::retry_limit_for_response_stream_error(&rules, 5, &CodexErr::ServerOverloaded),
        Some(2)
    );
}

#[test]
fn configured_retry_limit_is_capped() {
    let rules = [rule(&["server_overloaded"], &[], 101)];
    assert_eq!(
        super::retry_limit_for_response_stream_error(&rules, 5, &CodexErr::ServerOverloaded),
        Some(100)
    );
}

#[test]
fn empty_selectors_do_not_match() {
    let rules = [rule(&["  "], &[""], 9)];
    assert_eq!(
        super::retry_limit_for_response_stream_error(&rules, 5, &CodexErr::ServerOverloaded),
        Some(5)
    );
}

#[tokio::test]
async fn sampling_retry_logs_stream_error_context() {
    let (_session, turn_context) = make_session_and_context().await;
    let buffer: &'static std::sync::Mutex<Vec<u8>> =
        Box::leak(Box::new(std::sync::Mutex::new(Vec::new())));
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(tracing::Level::WARN)
        .with_writer(MockWriter::new(buffer))
        .finish();
    let _subscriber_guard = tracing::subscriber::set_default(subscriber);

    log_retry(
        ResponsesStreamRequest::Sampling,
        &turn_context,
        &CodexErr::Stream("websocket closed by server before response.completed".to_string()),
        /*retries*/ 2,
        /*max_retries*/ 5,
        Duration::from_secs(1),
    );

    let logs = String::from_utf8(
        buffer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone(),
    )
    .expect("retry log should be valid utf-8");
    assert!(logs.contains("stream disconnected - retrying sampling request"));
    assert!(logs.contains(&format!("turn_id={}", turn_context.sub_id)));
    assert!(logs.contains("retries=2"));
    assert!(logs.contains("max_retries=5"));
    assert!(logs.contains(
        "sampling_error=stream disconnected before completion: websocket closed by server before response.completed"
    ));
}
