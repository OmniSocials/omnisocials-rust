//! Limits and feature requests: param serialization (left-out fields stay
//! out, `source` is never sent) and the requests `limits()` and
//! `feature_requests()` send, against a tiny std `TcpListener` HTTP stub.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread::JoinHandle;

use omnisocials::{Client, CreateFeatureRequestParams, Error, ListLimitsParams};
use serde_json::{json, Value};

// ─── Params ──────────────────────────────────────────────────────────────────

#[test]
fn create_feature_request_params_drop_none_fields() {
    let only_summary = serde_json::to_value(CreateFeatureRequestParams {
        summary: "Reply to Google reviews from the inbox.".into(),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(only_summary, json!({ "summary": "Reply to Google reviews from the inbox." }));

    let full = serde_json::to_value(CreateFeatureRequestParams {
        summary: "Post our daily update to the Announcements topic.".into(),
        title: Some("Telegram topics".into()),
        platform: Some("telegram".into()),
        area: Some("posts".into()),
        limit_id: Some("telegram.forum_topics".into()),
    })
    .unwrap();
    assert_eq!(
        full,
        json!({
            "summary": "Post our daily update to the Announcements topic.",
            "title": "Telegram topics",
            "platform": "telegram",
            "area": "posts",
            "limit_id": "telegram.forum_topics"
        })
    );
    assert!(full.get("source").is_none());
}

// ─── Stub server ─────────────────────────────────────────────────────────────

fn http_response(status: u16, reason: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

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

fn client_for(base_url: &str) -> Client {
    Client::builder()
        .api_key("omsk_test_stub_key")
        .base_url(base_url)
        .max_retries(0)
        .build()
        .expect("build client")
}

fn body_of(request: &str) -> Value {
    let start = request.find("\r\n\r\n").unwrap() + 4;
    serde_json::from_str(&request[start..]).unwrap()
}

const LIMIT: &str = r#"{"id":"tiktok.delete","kind":"platform","kind_label":"The platform does not allow it","area":"posts","platforms":["tiktok"],"title":"Delete a TikTok video","says":"TikTok's API cannot delete a video.","instead":"Delete the video in the TikTok app.","requestable":false}"#;

const REQUEST: &str = r#"{"id":412,"title":"Post to a topic of a Telegram group","summary":"Post our daily update to the Announcements topic.","status":"open","team_answer":null,"team_replies":[],"votes_count":1,"limit_id":"telegram.forum_topics","platform":"telegram","area":"posts","source":"api","created_at":"2026-11-04T10:00:00.000Z","updated_at":"2026-11-04T10:00:00.000Z","completed_at":null}"#;

// ─── Requests ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn limits_and_feature_requests_send_each_call() {
    let (base_url, handle) = spawn_stub(vec![
        http_response(
            200,
            "OK",
            &format!(r#"{{"data":[{LIMIT}],"kinds":{{"platform":"The platform does not allow it"}}}}"#),
        ),
        http_response(200, "OK", r#"{"data":[],"kinds":{}}"#),
        http_response(201, "Created", &format!(r#"{{"outcome":"voted","data":{REQUEST}}}"#)),
        http_response(200, "OK", &format!(r#"{{"outcome":"already_requested","data":{REQUEST}}}"#)),
        http_response(200, "OK", &format!(r#"{{"data":[{REQUEST}]}}"#)),
        http_response(200, "OK", &format!(r#"{{"data":{REQUEST}}}"#)),
    ]);
    let client = client_for(&base_url);

    let limits = client
        .limits()
        .list(ListLimitsParams { platform: Some("tiktok".into()), area: Some("posts".into()) })
        .await
        .unwrap();
    client.limits().list(ListLimitsParams::default()).await.unwrap();
    let voted = client
        .feature_requests()
        .create(CreateFeatureRequestParams {
            summary: "Post our daily update to the Announcements topic.".into(),
            platform: Some("telegram".into()),
            limit_id: Some("telegram.forum_topics".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    let again = client
        .feature_requests()
        .create(CreateFeatureRequestParams { summary: "Post to a Telegram topic, please.".into(), ..Default::default() })
        .await
        .unwrap();
    let mine = client.feature_requests().list().await.unwrap();
    let one = client.feature_requests().get(412).await.unwrap();

    assert_eq!(limits["data"][0]["requestable"], false);
    assert_eq!(limits["kinds"]["platform"], "The platform does not allow it");
    assert_eq!(voted["outcome"], "voted");
    assert_eq!(voted["data"]["id"], 412);
    assert_eq!(again["outcome"], "already_requested");
    assert_eq!(mine["data"][0]["status"], "open");
    assert_eq!(one["data"]["limit_id"], "telegram.forum_topics");

    let captured = handle.join().unwrap();
    assert!(captured[0].starts_with("GET /limits?platform=tiktok&area=posts HTTP/1.1"), "{}", captured[0]);
    assert!(captured[1].starts_with("GET /limits HTTP/1.1"), "{}", captured[1]);
    assert!(captured[2].starts_with("POST /feature-requests HTTP/1.1"), "{}", captured[2]);
    assert_eq!(
        body_of(&captured[2]),
        json!({
            "summary": "Post our daily update to the Announcements topic.",
            "platform": "telegram",
            "limit_id": "telegram.forum_topics"
        })
    );
    assert!(captured[4].starts_with("GET /feature-requests HTTP/1.1"), "{}", captured[4]);
    assert!(captured[5].starts_with("GET /feature-requests/412 HTTP/1.1"), "{}", captured[5]);
}

#[tokio::test]
async fn not_requestable_maps_to_an_api_error_with_the_limit() {
    let (base_url, _handle) = spawn_stub(vec![http_response(
        409,
        "Conflict",
        &format!(
            r#"{{"error":{{"code":"not_requestable","message":"TikTok's API cannot delete a video. A request cannot change this, so nothing was sent.","limit":{LIMIT}}}}}"#
        ),
    )]);
    let client = client_for(&base_url);
    let err = client
        .feature_requests()
        .create(CreateFeatureRequestParams {
            summary: "Delete my TikTok videos from OmniSocials.".into(),
            limit_id: Some("tiktok.delete".into()),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert_eq!(err.code(), Some("not_requestable"));
    match err {
        Error::Api { status, body, .. } => {
            assert_eq!(status, 409);
            assert_eq!(body.unwrap()["error"]["limit"]["instead"], "Delete the video in the TikTok app.");
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}
