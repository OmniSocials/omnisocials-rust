//! Request-loop tests against a tiny std `TcpListener` HTTP stub: header
//! propagation, retry behavior, error mapping, 204 handling, and the
//! env-var / builder auth fallback.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread::JoinHandle;

use omnisocials::{Client, CreatePostParams, Error, ListPostsParams};

// ─── Stub server ─────────────────────────────────────────────────────────────

/// Build one canned HTTP/1.1 response with a correct Content-Length.
fn http_response(status: u16, reason: &str, extra_headers: &[(&str, &str)], body: &str) -> String {
    let mut response = format!("HTTP/1.1 {status} {reason}\r\n");
    for (name, value) in extra_headers {
        response.push_str(&format!("{name}: {value}\r\n"));
    }
    if status != 204 {
        response.push_str("Content-Type: application/json\r\n");
        response.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    response.push_str("Connection: close\r\n\r\n");
    if status != 204 {
        response.push_str(body);
    }
    response
}

/// Serve one canned response per incoming connection, capturing each raw
/// request. Returns the base URL and a handle resolving to the captures.
fn spawn_stub(responses: Vec<String>) -> (String, JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind stub listener");
    let base_url = format!("http://{}", listener.local_addr().unwrap());

    let handle = std::thread::spawn(move || {
        let mut captured = Vec::new();
        for response in responses {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut raw = Vec::new();
            let mut buf = [0u8; 16 * 1024];
            loop {
                let n = stream.read(&mut buf).expect("read request");
                if n == 0 {
                    break;
                }
                raw.extend_from_slice(&buf[..n]);
                if let Some(headers_end) = find_subslice(&raw, b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&raw[..headers_end]).to_lowercase();
                    let content_length = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length:"))
                        .and_then(|value| value.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    if raw.len() >= headers_end + 4 + content_length {
                        break;
                    }
                }
            }
            captured.push(String::from_utf8_lossy(&raw).to_string());
            stream.write_all(response.as_bytes()).expect("write response");
            stream.flush().ok();
        }
        captured
    });

    (base_url, handle)
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

fn client_for(base_url: &str, max_retries: u32) -> Client {
    Client::builder()
        .api_key("omsk_test_stub_key")
        .base_url(base_url)
        .max_retries(max_retries)
        .build()
        .expect("build client")
}

// ─── Happy path + headers ────────────────────────────────────────────────────

#[tokio::test]
async fn get_sends_auth_and_user_agent_and_returns_body() {
    let (base_url, handle) = spawn_stub(vec![http_response(
        200,
        "OK",
        &[],
        r#"{"data":[{"id":"1"}],"pagination":{"total":1}}"#,
    )]);

    let client = client_for(&base_url, 0);
    let result = client
        .posts()
        .list(ListPostsParams {
            status: Some("scheduled".into()),
            limit: Some(50),
            ..Default::default()
        })
        .await
        .unwrap();

    // Body returned as-is, envelope not unwrapped.
    assert_eq!(result["data"][0]["id"], "1");
    assert_eq!(result["pagination"]["total"], 1);

    let captured = handle.join().unwrap();
    let request = captured[0].to_lowercase();
    assert!(request.starts_with("get /posts?status=scheduled&limit=50 http/1.1"), "{request}");
    assert!(request.contains("authorization: bearer omsk_test_stub_key"), "{request}");
    assert!(
        request.contains(&format!("user-agent: omnisocials-rust/{}", omnisocials::VERSION)),
        "{request}"
    );
}

#[tokio::test]
async fn post_body_drops_none_fields_on_the_wire() {
    let (base_url, handle) =
        spawn_stub(vec![http_response(201, "Created", &[], r#"{"data":{"id":"9"}}"#)]);

    let client = client_for(&base_url, 0);
    client
        .posts()
        .create(CreatePostParams {
            content: "Wire test".into(),
            channels: Some(vec!["bluesky".into()]),
            ..Default::default()
        })
        .await
        .unwrap();

    let captured = handle.join().unwrap();
    let body_start = captured[0].find("\r\n\r\n").unwrap() + 4;
    let body: serde_json::Value = serde_json::from_str(&captured[0][body_start..]).unwrap();
    assert_eq!(body, serde_json::json!({"content": "Wire test", "channels": ["bluesky"]}));
    assert!(captured[0].to_lowercase().contains("content-type: application/json"));
}

#[tokio::test]
async fn delete_204_returns_null() {
    let (base_url, handle) = spawn_stub(vec![http_response(204, "No Content", &[], "")]);

    let client = client_for(&base_url, 0);
    let result = client.posts().delete("abc/123").await.unwrap();
    assert!(result.is_null());

    let captured = handle.join().unwrap();
    // Path segment must be percent-encoded.
    assert!(captured[0].starts_with("DELETE /posts/abc%2F123 HTTP/1.1"), "{}", captured[0]);
}

// ─── Retries ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn retries_on_5xx_then_succeeds() {
    let (base_url, handle) = spawn_stub(vec![
        http_response(500, "Internal Server Error", &[], r#"{"error":{"code":"internal_error","message":"boom"}}"#),
        http_response(200, "OK", &[], r#"{"data":{"ok":true}}"#),
    ]);

    let client = client_for(&base_url, 2);
    let result = client.health().await.unwrap();
    assert_eq!(result["data"]["ok"], true);
    assert_eq!(handle.join().unwrap().len(), 2);
}

#[tokio::test]
async fn retries_on_429_honoring_retry_after() {
    let (base_url, handle) = spawn_stub(vec![
        http_response(429, "Too Many Requests", &[("Retry-After", "0")], r#"{"error":{"code":"rate_limit_exceeded","message":"slow down"}}"#),
        http_response(200, "OK", &[], r#"{"data":{"ok":true}}"#),
    ]);

    let client = client_for(&base_url, 1);
    let result = client.accounts().list().await.unwrap();
    assert_eq!(result["data"]["ok"], true);
    assert_eq!(handle.join().unwrap().len(), 2);
}

#[tokio::test]
async fn does_not_retry_4xx_and_maps_validation_error() {
    let (base_url, handle) = spawn_stub(vec![http_response(
        422,
        "Unprocessable Entity",
        &[],
        r#"{"error":{"code":"validation_error","message":"content is required"}}"#,
    )]);

    let client = client_for(&base_url, 3);
    let err = client.folders().list().await.unwrap_err();
    match &err {
        Error::Validation { status, code, message, body } => {
            assert_eq!(*status, 422);
            assert_eq!(code.as_deref(), Some("validation_error"));
            assert_eq!(message, "content is required");
            assert_eq!(body.as_ref().unwrap()["error"]["code"], "validation_error");
        }
        other => panic!("expected Validation, got {other:?}"),
    }
    // Exactly one request despite max_retries = 3: 4xx is never retried.
    assert_eq!(handle.join().unwrap().len(), 1);
}

#[tokio::test]
async fn connection_error_after_retries_exhausted() {
    // Nothing is listening on this port (bind then drop to reserve-and-free).
    let unused = TcpListener::bind("127.0.0.1:0").unwrap();
    let base_url = format!("http://{}", unused.local_addr().unwrap());
    drop(unused);

    let client = client_for(&base_url, 0);
    let err = client.health().await.unwrap_err();
    assert!(matches!(err, Error::Connection(_)), "expected Connection, got {err:?}");
}

// ─── Error mapping ───────────────────────────────────────────────────────────

#[tokio::test]
async fn maps_status_codes_to_error_variants() {
    let cases: Vec<(u16, &str)> = vec![
        (400, "Bad Request"),
        (401, "Unauthorized"),
        (403, "Forbidden"),
        (404, "Not Found"),
        (409, "Conflict"),
        (429, "Too Many Requests"),
        (500, "Internal Server Error"),
    ];
    let responses = cases
        .iter()
        .map(|(status, reason)| {
            let extra: &[(&str, &str)] =
                if *status == 429 { &[("Retry-After", "7")] } else { &[] };
            http_response(*status, reason, extra, r#"{"error":{"code":"some_code","message":"nope"}}"#)
        })
        .collect();
    let (base_url, handle) = spawn_stub(responses);

    let client = client_for(&base_url, 0);
    let mut errors = Vec::new();
    for _ in &cases {
        errors.push(client.health().await.unwrap_err());
    }
    handle.join().unwrap();

    assert!(matches!(errors[0], Error::Validation { status: 400, .. }), "{:?}", errors[0]);
    assert!(matches!(errors[1], Error::Auth { status: 401, .. }), "{:?}", errors[1]);
    assert!(matches!(errors[2], Error::PermissionDenied { status: 403, .. }), "{:?}", errors[2]);
    assert!(matches!(errors[3], Error::NotFound { status: 404, .. }), "{:?}", errors[3]);
    assert!(matches!(errors[4], Error::Api { status: 409, .. }), "{:?}", errors[4]);
    match &errors[5] {
        Error::RateLimit { retry_after, code, .. } => {
            assert_eq!(*retry_after, Some(7.0));
            assert_eq!(code.as_deref(), Some("some_code"));
        }
        other => panic!("expected RateLimit, got {other:?}"),
    }
    assert!(matches!(errors[6], Error::Server { status: 500, .. }), "{:?}", errors[6]);
}

// ─── Env fallback + builder precedence ───────────────────────────────────────
//
// Everything touching OMNISOCIALS_API_KEY lives in this single test so that
// parallel test threads never race on the process environment (all other
// tests pass an explicit api_key and never read the env).

#[test]
fn env_fallback_and_builder_precedence() {
    const ENV_KEY: &str = "OMNISOCIALS_API_KEY";

    // 1. No key anywhere: construction fails with Error::Auth.
    std::env::remove_var(ENV_KEY);
    match Client::from_env() {
        Err(Error::Auth { status, code, message, .. }) => {
            assert_eq!(status, 401);
            assert_eq!(code.as_deref(), Some("missing_api_key"));
            assert!(message.contains(ENV_KEY));
        }
        other => panic!("expected Error::Auth, got {other:?}"),
    }
    assert!(matches!(Client::builder().build(), Err(Error::Auth { .. })));

    // 2. Env var set: from_env and a key-less builder succeed.
    std::env::set_var(ENV_KEY, "omsk_test_from_env");
    assert!(Client::from_env().is_ok());
    assert!(Client::builder().build().is_ok());

    // 3. Explicit builder key wins over the env var (verified on the wire).
    let (base_url, handle) = spawn_stub(vec![http_response(200, "OK", &[], r#"{"status":"ok"}"#)]);
    let client = Client::builder()
        .api_key("omsk_test_explicit")
        .base_url(&base_url)
        .timeout(std::time::Duration::from_secs(5))
        .max_retries(0)
        .build()
        .unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let health = runtime.block_on(client.health()).unwrap();
    assert_eq!(health["status"], "ok");
    let request = handle.join().unwrap()[0].to_lowercase();
    assert!(request.contains("authorization: bearer omsk_test_explicit"), "{request}");
    assert!(!request.contains("omsk_test_from_env"), "{request}");

    // 4. An empty env var is treated as missing.
    std::env::set_var(ENV_KEY, "");
    assert!(matches!(Client::from_env(), Err(Error::Auth { .. })));

    std::env::remove_var(ENV_KEY);
}
