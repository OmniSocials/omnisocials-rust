//! Post tags: param serialization (ids and names mixed, explicit `null` to
//! clear) and the requests the `tags()` resource and `analytics().tags()`
//! send, against a tiny std `TcpListener` HTTP stub.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread::JoinHandle;

use omnisocials::{
    AnalyticsOverviewParams, ApplyTagsParams, Client, CreatePostParams, CreateTagParams, Error,
    ListPostsParams, TagAnalyticsParams, TagRef, UpdatePostParams, UpdateTagParams,
};
use serde_json::{json, Value};

// ─── Params ──────────────────────────────────────────────────────────────────

#[test]
fn tag_refs_serialize_as_ids_or_names() {
    let refs: Vec<TagRef> = vec![12.into(), "Question".into(), String::from("Talking head").into()];
    assert_eq!(serde_json::to_value(&refs).unwrap(), json!([12, "Question", "Talking head"]));
}

#[test]
fn create_post_params_carry_tags() {
    let params = CreatePostParams {
        content: "Hi".into(),
        tags: Some(vec!["Question".into(), 14.into()]),
        ..Default::default()
    };
    let value = serde_json::to_value(&params).unwrap();
    assert_eq!(value["tags"], json!(["Question", 14]));

    let without = serde_json::to_value(CreatePostParams { content: "Hi".into(), ..Default::default() }).unwrap();
    assert!(without.get("tags").is_none());
}

#[test]
fn update_post_params_clear_or_replace_tags() {
    let cleared = serde_json::to_value(UpdatePostParams { tags: Some(None), ..Default::default() }).unwrap();
    assert_eq!(cleared, json!({ "tags": null }));

    let empty = serde_json::to_value(UpdatePostParams { tags: Some(Some(vec![])), ..Default::default() }).unwrap();
    assert_eq!(empty, json!({ "tags": [] }));

    let replaced =
        serde_json::to_value(UpdatePostParams { tags: Some(Some(vec![12.into()])), ..Default::default() }).unwrap();
    assert_eq!(replaced, json!({ "tags": [12] }));

    let untouched = serde_json::to_value(UpdatePostParams::default()).unwrap();
    assert_eq!(untouched, json!({}));
}

#[test]
fn tag_params_drop_none_fields() {
    let create = serde_json::to_value(CreateTagParams { name: "Question".into(), ..Default::default() }).unwrap();
    assert_eq!(create, json!({ "name": "Question" }));

    let update = serde_json::to_value(UpdateTagParams { color: Some("green".into()), ..Default::default() }).unwrap();
    assert_eq!(update, json!({ "color": "green" }));

    let apply = serde_json::to_value(ApplyTagsParams {
        post_ids: vec!["1024".into()],
        set: Some(vec![]),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(apply, json!({ "post_ids": ["1024"], "set": [] }));
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

const TAG: &str = r#"{"id":12,"name":"Question","color":"blue","created_at":"2026-11-02T10:00:00Z"}"#;

// ─── Requests ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn tags_resource_sends_each_call() {
    let (base_url, handle) = spawn_stub(vec![
        http_response(200, "OK", &format!(r#"{{"data":[{TAG}]}}"#)),
        http_response(201, "Created", &format!(r#"{{"data":{TAG}}}"#)),
        http_response(200, "OK", &format!(r#"{{"data":{TAG}}}"#)),
        http_response(200, "OK", &format!(r#"{{"data":{{"tag":{TAG},"posts_untagged":7}}}}"#)),
        http_response(
            200,
            "OK",
            &format!(r#"{{"data":{{"updated":1,"posts":[{{"id":"1024","tags":[{TAG}]}}],"not_found":["9"]}}}}"#),
        ),
    ]);
    let client = client_for(&base_url);
    let tags = client.tags();

    let list = tags.list().await.unwrap();
    let created = tags
        .create(CreateTagParams { name: "Question".into(), color: Some("blue".into()) })
        .await
        .unwrap();
    tags.update(12, UpdateTagParams { name: Some("Question hook".into()), ..Default::default() })
        .await
        .unwrap();
    let deleted = tags.delete(12).await.unwrap();
    let applied = tags
        .apply(ApplyTagsParams {
            post_ids: vec!["1024".into(), "9".into()],
            add: Some(vec![12.into(), "Talking head".into()]),
            remove: Some(vec!["Old".into()]),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(list["data"][0]["name"], "Question");
    assert_eq!(created["data"]["id"], 12);
    assert_eq!(deleted["data"]["posts_untagged"], 7);
    assert_eq!(applied["data"]["not_found"], json!(["9"]));

    let captured = handle.join().unwrap();
    assert!(captured[0].starts_with("GET /tags HTTP/1.1"), "{}", captured[0]);
    assert!(captured[1].starts_with("POST /tags HTTP/1.1"), "{}", captured[1]);
    assert_eq!(body_of(&captured[1]), json!({ "name": "Question", "color": "blue" }));
    assert!(captured[2].starts_with("PATCH /tags/12 HTTP/1.1"), "{}", captured[2]);
    assert_eq!(body_of(&captured[2]), json!({ "name": "Question hook" }));
    assert!(captured[3].starts_with("DELETE /tags/12 HTTP/1.1"), "{}", captured[3]);
    assert!(captured[4].starts_with("POST /tags/apply HTTP/1.1"), "{}", captured[4]);
    assert_eq!(
        body_of(&captured[4]),
        json!({ "post_ids": ["1024", "9"], "add": [12, "Talking head"], "remove": ["Old"] })
    );
}

#[tokio::test]
async fn analytics_tags_and_tag_filters_build_the_query() {
    let (base_url, handle) = spawn_stub(vec![
        http_response(
            200,
            "OK",
            r#"{"data":{"period":{"start_date":"2026-08-01T00:00:00.000Z","end_date":"2026-10-31T23:59:59.999Z"},"platform":"tiktok","min_posts":3,"tags":[{"tag":{"id":12,"name":"Question","color":"blue"},"posts":6,"measured":5,"median_impressions":1200,"engagement_rate":5.4,"vs_normal":0.38,"best_channel":"tiktok","enough_posts":true}],"untagged":{"posts":4,"measured":4,"median_impressions":null,"engagement_rate":null,"vs_normal":null,"best_channel":null,"enough_posts":true},"normals":{"tiktok":870}}}"#,
        ),
        http_response(200, "OK", r#"{"data":[],"pagination":{"total":0}}"#),
        http_response(200, "OK", r#"{"data":{"total_posts":0}}"#),
    ]);
    let client = client_for(&base_url);

    let compared = client
        .analytics()
        .tags(TagAnalyticsParams {
            start_date: Some("2026-08-01".into()),
            end_date: Some("2026-10-31".into()),
            platform: Some("tiktok".into()),
            min_posts: Some(3),
        })
        .await
        .unwrap();
    client
        .posts()
        .list(ListPostsParams { tag: Some("Question,14".into()), ..Default::default() })
        .await
        .unwrap();
    client
        .analytics()
        .overview(AnalyticsOverviewParams {
            period: Some("30d".into()),
            tag: Some("12".into()),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(compared["data"]["tags"][0]["tag"]["name"], "Question");
    assert_eq!(compared["data"]["tags"][0]["vs_normal"], 0.38);
    assert!(compared["data"]["untagged"]["vs_normal"].is_null());

    let captured = handle.join().unwrap();
    assert!(
        captured[0].starts_with(
            "GET /analytics/tags?start_date=2026-08-01&end_date=2026-10-31&platform=tiktok&min_posts=3 HTTP/1.1"
        ),
        "{}",
        captured[0]
    );
    assert!(captured[1].starts_with("GET /posts?tag=Question%2C14 HTTP/1.1"), "{}", captured[1]);
    assert!(captured[2].starts_with("GET /analytics/overview?period=30d&tag=12 HTTP/1.1"), "{}", captured[2]);
}

#[tokio::test]
async fn tag_errors_map_to_api_errors() {
    let (base_url, _handle) = spawn_stub(vec![http_response(
        409,
        "Conflict",
        &format!(r#"{{"error":{{"code":"tag_exists","message":"A tag with this name exists.","tag":{TAG}}}}}"#),
    )]);
    let client = client_for(&base_url);
    let err = client
        .tags()
        .create(CreateTagParams { name: "question".into(), ..Default::default() })
        .await
        .unwrap_err();
    match err {
        Error::Api { status, code, body, .. } => {
            assert_eq!(status, 409);
            assert_eq!(code.as_deref(), Some("tag_exists"));
            assert_eq!(body.unwrap()["error"]["tag"]["id"], 12);
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}
