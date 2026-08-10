# Configuration

For basic configuration instructions, see [this documentation](https://developers.openai.com/codex/config-basic).

For advanced configuration instructions, see [this documentation](https://developers.openai.com/codex/config-advanced).

For a full configuration reference, see [this documentation](https://developers.openai.com/codex/config-reference).

## Stream retry overrides

Codex retries transient Responses stream failures using the selected provider's
`stream_max_retries` value. Ordered `stream_retry_rules` can override that limit
for a structured Codex error code or a case-insensitive message substring. The
first matching rule wins, `max_retries = 0` disables retry for that match, and
values above 100 are capped at 100.

Rules cannot turn terminal errors such as authentication, quota, policy,
invalid-request, or permission failures into retryable errors.

```toml
[[stream_retry_rules]]
error_codes = ["server_overloaded"]
max_retries = 8

[[stream_retry_rules]]
message_contains = ["selected model is at capacity"]
max_retries = 8
```

## Lifecycle hooks

Admins can set top-level `allow_managed_hooks_only = true` in
`requirements.toml` to ignore user, project, and session hook configs while
still allowing managed hooks from requirements and managed config layers. This
setting is only supported in `requirements.toml`; putting it in `config.toml`
does not enable managed-hooks-only mode.
