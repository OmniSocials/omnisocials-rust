//! X statistics on credits: the `x_statistics` body of `accounts().update`
//! (left-out fields stay out, `None` is sent as null), the requests of
//! `analytics().refresh_post` and `credits().get`, and the retry rule of a
//! refresh (sent again after a 429 only), against a tiny std `TcpListener`
//! HTTP stub.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread::JoinHandle;

use omnisocials::{
    Client, Error, UpdateAccountParams, XStatisticsInput, XStatisticsPostSchedule,
    X_STATISTICS_WEBHOOK_EVENTS,
};
use serde_json::{json, Value};

// ─── Params ──────────────────────────────────────────────────────────────────

#[test]
fn update_account_params_send_only_the_given_fields_and_null() {
    let both = serde_json::to_value(UpdateAccountParams {
        x_statistics: Some(XStatisticsInput {
            posts: Some("light".into()),
            followers: Some("daily".into()),
            ..Default::default()
        }),
    })
    .unwrap();
    assert_eq!(both, json!({ "x_statistics": { "posts": "light", "followers": "daily" } }));

    let one = serde_json::to_value(UpdateAccountParams {
        x_statistics: Some(XStatisticsInput { followers: Some("off".into()), ..Default::default() }),
    })
    .unwrap();
    assert_eq!(one, json!({ "x_statistics": { "followers": "off" } }));

    let custom = serde_json::to_value(UpdateAccountParams {
        x_statistics: Some(XStatisticsInput {
            posts: Some("custom".into()),
            post_schedule: Some(XStatisticsPostSchedule {
                at: vec!["1h".into(), "1d".into()],
                then_every: Some("7d".into()),
                until: None,
            }),
            ..Default::default()
        }),
    })
    .unwrap();
    assert_eq!(
        custom,
        json!({ "x_statistics": { "posts": "custom", "post_schedule": { "at": ["1h", "1d"], "then_every": "7d" } } })
    );

    let null = serde_json::to_value(UpdateAccountParams::default()).unwrap();
    assert_eq!(null, json!({ "x_statistics": null }));
}

#[test]
fn webhook_event_names() {
    assert_eq!(X_STATISTICS_WEBHOOK_EVENTS, ["x_statistics.paused", "x_statistics.resumed"]);
}

// ─── Stub server ─────────────────────────────────────────────────────────────

fn http_response(status: u16, reason: &str, extra_headers: &[(&str, &str)], body: &str) -> String {
    let mut response = format!("HTTP/1.1 {status} {reason}\r\n");
    for (name, value) in extra_headers {
        response.push_str(&format!("{name}: {value}\r\n"));
    }
    response.push_str("Content-Type: application/json\r\n");
    response.push_str(&format!("Content-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()));
    response
}

/// One canned response per connection (an empty one closes the connection
/// without an answer); resolves to the raw requests.
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
                if let Some(end) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&raw[..end]).to_lowercase();
                    let length = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length:"))
                        .and_then(|value| value.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    if raw.len() >= end + 4 + length {
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

fn client_for(base_url: &str, max_retries: u32) -> Client {
    Client::builder()
        .api_key("omsk_test_stub_key")
        .base_url(base_url)
        .max_retries(max_retries)
        .build()
        .expect("build client")
}

fn request_line(raw: &str) -> &str {
    raw.split("\r\n").next().unwrap_or("")
}

fn body_of(raw: &str) -> String {
    raw[raw.find("\r\n\r\n").unwrap() + 4..].to_string()
}

const X_ACCOUNT: &str = r#"{"data":{"id":"1234_x","platform":"x","x_statistics":{"posts":"light","followers":"daily","post_schedule":null,"source":"account","billing":"credits","state":"paused","paused_reason":"insufficient_credits"}}}"#;

// ─── Requests ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn update_refresh_and_credits_send_each_call() {
    let (base_url, handle) = spawn_stub(vec![
        http_response(200, "OK", &[], X_ACCOUNT),
        http_response(200, "OK", &[], X_ACCOUNT),
        http_response(
            200,
            "OK",
            &[],
            r#"{"data":{"post_id":9123,"tweets":[{"id":"1859","metrics":{"views":3412},"read_at":"2027-01-08T09:12:04Z"}],"credits_used":0.5,"read_at":"2027-01-08T09:12:04Z"}}"#,
        ),
        http_response(
            200,
            "OK",
            &[],
            r#"{"data":{"balance":412,"auto_refill":{"enabled":false},"this_month":{"x_statistics":31},"x_statistics":{"state":"active","paused_at":null,"charging":{"active":true,"from":"2027-01-01T00:00:00.000Z"},"prices":{"post_read":0.5,"media_read":0.5,"follower_read":1},"total_credits_month":38,"this_month":{"post_reads":40,"media_reads":2,"follower_reads":8,"refreshes":1,"credits":31},"accounts":[{"account_id":"1234_x","workspace_id":1234,"handle":"@omnisocials","posts":"full","followers":"daily","tweets_30d":12,"credits_month":38}]}}}"#,
        ),
    ]);
    let client = client_for(&base_url, 0);

    let updated = client
        .accounts()
        .update(
            "1234_x",
            UpdateAccountParams {
                x_statistics: Some(XStatisticsInput {
                    posts: Some("light".into()),
                    followers: Some("daily".into()),
                    ..Default::default()
                }),
            },
        )
        .await
        .unwrap();
    client.accounts().update("1234_x", UpdateAccountParams { x_statistics: None }).await.unwrap();
    let fresh = client.analytics().refresh_post("9123").await.unwrap();
    let credits = client.credits().get().await.unwrap();

    assert_eq!(updated["data"]["x_statistics"]["state"], "paused");
    assert_eq!(fresh["data"]["credits_used"], 0.5);
    assert_eq!(fresh["data"]["tweets"][0]["id"], "1859");
    assert_eq!(credits["data"]["balance"], 412);
    assert_eq!(credits["data"]["x_statistics"]["prices"]["follower_read"], 1);

    let captured = handle.join().unwrap();
    assert_eq!(request_line(&captured[0]), "PATCH /accounts/1234_x HTTP/1.1");
    let sent: Value = serde_json::from_str(&body_of(&captured[0])).unwrap();
    assert_eq!(sent, json!({ "x_statistics": { "posts": "light", "followers": "daily" } }));
    assert_eq!(body_of(&captured[1]), r#"{"x_statistics":null}"#);
    assert_eq!(request_line(&captured[2]), "POST /analytics/posts/9123/refresh HTTP/1.1");
    assert_eq!(body_of(&captured[2]), "");
    assert_eq!(request_line(&captured[3]), "GET /credits HTTP/1.1");
}

// A refresh can cost credits: after a 502 or a dropped connection the
// first attempt may have gone through, so it is not sent again. The stub
// has one extra slot per call that a retry would take; the health check
// after each call proves there was none.
#[tokio::test]
async fn refresh_post_is_not_retried_after_a_502_or_a_dropped_connection() {
    let healthy = || http_response(200, "OK", &[], r#"{"status":"ok"}"#);
    let (base_url, handle) = spawn_stub(vec![
        http_response(
            502,
            "Bad Gateway",
            &[("Retry-After", "0")],
            r#"{"error":{"code":"refresh_failed","message":"X did not answer. Nothing was charged."}}"#,
        ),
        healthy(),
        String::new(),
        healthy(),
    ]);
    let client = client_for(&base_url, 2);

    let err = client.analytics().refresh_post("9123").await.unwrap_err();
    assert!(matches!(err, Error::Server { status: 502, .. }), "{err:?}");
    assert_eq!(err.code(), Some("refresh_failed"));
    client.health().await.unwrap();

    let err = client.analytics().refresh_post("9123").await.unwrap_err();
    assert!(matches!(err, Error::Connection(_)), "expected Connection, got {err:?}");
    client.health().await.unwrap();

    let lines: Vec<String> = handle.join().unwrap().iter().map(|raw| request_line(raw).to_owned()).collect();
    assert_eq!(
        lines,
        vec![
            "POST /analytics/posts/9123/refresh HTTP/1.1",
            "GET /health HTTP/1.1",
            "POST /analytics/posts/9123/refresh HTTP/1.1",
            "GET /health HTTP/1.1",
        ]
    );
}

#[tokio::test]
async fn refresh_post_is_retried_after_a_429() {
    let (base_url, handle) = spawn_stub(vec![
        http_response(
            429,
            "Too Many Requests",
            &[("Retry-After", "0")],
            r#"{"error":{"code":"rate_limit_exceeded","message":"slow down"}}"#,
        ),
        http_response(200, "OK", &[], r#"{"data":{"post_id":9123,"tweets":[],"credits_used":0}}"#),
    ]);
    let client = client_for(&base_url, 2);

    let fresh = client.analytics().refresh_post("9123").await.unwrap();
    assert_eq!(fresh["data"]["credits_used"], 0);
    let captured = handle.join().unwrap();
    assert_eq!(captured.len(), 2);
    assert_eq!(request_line(&captured[1]), "POST /analytics/posts/9123/refresh HTTP/1.1");
}

#[tokio::test]
async fn insufficient_credits_is_a_402_api_error() {
    let (base_url, _handle) = spawn_stub(vec![http_response(
        402,
        "Payment Required",
        &[],
        r#"{"error":{"code":"insufficient_credits","message":"Not enough credits"}}"#,
    )]);
    let client = client_for(&base_url, 2);
    let err = client.analytics().refresh_post("9123").await.unwrap_err();
    assert_eq!(err.code(), Some("insufficient_credits"));
    assert!(matches!(err, Error::Api { status: 402, .. }), "{err:?}");
}
