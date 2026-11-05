//! Request-loop tests against a tiny std `TcpListener` HTTP stub: header
//! propagation, retry behavior, error mapping, 204 handling, and the
//! env-var / builder auth fallback.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread::JoinHandle;

use omnisocials::{
    AdBudget, AdDraftAd, AdDraftParams, AdDraftSchedule, AdDraftTargeting, AdMediaFile, Client,
    CreatePostParams, Error, LaunchAdDraftParams, ListPinterestProductsParams, ListPostsParams,
    SearchLocationsParams, UpdateAdCampaignParams,
};

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
/// request. An empty response closes the connection without an answer.
/// Returns the base URL and a handle resolving to the captures.
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
async fn posts_get_approval_reads_the_review() {
    let (base_url, handle) = spawn_stub(vec![http_response(
        200,
        "OK",
        &[],
        r#"{"data":{"post_id":"123456","status":"rejected","workflow":{"id":"42","name":"Content approval"},"current_step":null,"steps":[{"order":1,"name":"Client approval","require_mode":"all","status":"rejected","approvers":[{"id":"c4a09e1d","name":"Jordan","email":null,"status":"rejected","decided_at":"2026-10-01T14:30:00.000Z","comment":"Wrong product photo"}]}],"rejection":{"by":{"id":"c4a09e1d","name":"Jordan"},"reason":"Wrong product photo","at":"2026-10-01T14:30:00.000Z","step":1},"comments":[]}}"#,
    )]);

    let client = client_for(&base_url, 0);
    let result = client.posts().get_approval("123456").await.unwrap();
    assert_eq!(result["data"]["status"], "rejected");
    assert!(result["data"]["current_step"].is_null());
    assert_eq!(result["data"]["steps"][0]["approvers"][0]["status"], "rejected");
    assert_eq!(result["data"]["rejection"]["by"]["id"], "c4a09e1d");
    assert_eq!(result["data"]["rejection"]["reason"], "Wrong product photo");

    let captured = handle.join().unwrap();
    assert!(captured[0].starts_with("GET /posts/123456/approval HTTP/1.1"), "{}", captured[0]);
}

#[tokio::test]
async fn inbox_hide_posts_the_hide_flag() {
    let (base_url, handle) = spawn_stub(vec![http_response(
        200,
        "OK",
        &[],
        r#"{"data":{"id":"123","hidden":true}}"#,
    )]);

    let client = client_for(&base_url, 0);
    let result = client.inbox().hide("123", true).await.unwrap();
    assert_eq!(result["data"]["hidden"], true);

    let captured = handle.join().unwrap();
    assert!(captured[0].starts_with("POST /inbox/messages/123/hide HTTP/1.1"), "{}", captured[0]);
    let body_start = captured[0].find("\r\n\r\n").unwrap() + 4;
    let body: serde_json::Value = serde_json::from_str(&captured[0][body_start..]).unwrap();
    assert_eq!(body, serde_json::json!({"hide": true}));
}

#[tokio::test]
async fn inbox_like_posts_the_like_flag() {
    let (base_url, handle) = spawn_stub(vec![http_response(
        200,
        "OK",
        &[],
        r#"{"data":{"id":"123","liked":false}}"#,
    )]);

    let client = client_for(&base_url, 0);
    let result = client.inbox().like("123", false).await.unwrap();
    assert_eq!(result["data"]["liked"], false);

    let captured = handle.join().unwrap();
    assert!(captured[0].starts_with("POST /inbox/messages/123/like HTTP/1.1"), "{}", captured[0]);
    let body_start = captured[0].find("\r\n\r\n").unwrap() + 4;
    let body: serde_json::Value = serde_json::from_str(&captured[0][body_start..]).unwrap();
    assert_eq!(body, serde_json::json!({"like": false}));
}

#[tokio::test]
async fn locations_search_with_builds_platform_and_coordinate_queries() {
    let (base_url, handle) = spawn_stub(vec![
        http_response(200, "OK", &[], r#"{"locations":[]}"#),
        http_response(200, "OK", &[], r#"{"locations":[]}"#),
    ]);

    let client = client_for(&base_url, 0);
    client
        .locations()
        .search_with(SearchLocationsParams {
            q: Some("cafe".into()),
            platform: Some("threads".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    client
        .locations()
        .search_with(SearchLocationsParams {
            platform: Some("threads".into()),
            latitude: Some(52.37),
            longitude: Some(-4.89),
            ..Default::default()
        })
        .await
        .unwrap();

    let captured = handle.join().unwrap();
    assert!(
        captured[0].starts_with("GET /locations/search?q=cafe&platform=threads HTTP/1.1"),
        "{}",
        captured[0]
    );
    // Coordinates go on the wire with plain decimal points.
    assert!(
        captured[1]
            .starts_with("GET /locations/search?platform=threads&latitude=52.37&longitude=-4.89 HTTP/1.1"),
        "{}",
        captured[1]
    );
}

#[tokio::test]
async fn pinterest_list_products_builds_the_query_and_validate_sends_the_id() {
    let (base_url, handle) = spawn_stub(vec![
        http_response(
            200,
            "OK",
            &[],
            r#"{"products":[{"pin_id":"813744226420795884","price":24.99}],"bookmark":null,"source":"catalog","catalog_access":true}"#,
        ),
        http_response(
            200,
            "OK",
            &[],
            r#"{"error":{"code":"pinterest_not_connected","message":"No Pinterest account is connected."}}"#,
        ),
        http_response(200, "OK", &[], r#"{"valid":true,"pin_id":"813744226420795884"}"#),
    ]);

    let client = client_for(&base_url, 0);
    let list = client
        .pinterest()
        .list_products(ListPinterestProductsParams {
            source: Some("catalog".into()),
            product_group_id: Some("443727193917".into()),
            bookmark: Some("abc".into()),
            page_size: Some(50),
        })
        .await
        .unwrap();
    let not_connected = client
        .pinterest()
        .list_products(ListPinterestProductsParams::default())
        .await
        .unwrap();
    let check = client.pinterest().validate_product("813744226420795884").await.unwrap();

    let captured = handle.join().unwrap();
    assert!(
        captured[0].starts_with(
            "GET /pinterest/products?source=catalog&product_group_id=443727193917&bookmark=abc&page_size=50 HTTP/1.1"
        ),
        "{}",
        captured[0]
    );
    assert_eq!(list["products"][0]["pin_id"], "813744226420795884");
    assert_eq!(list["catalog_access"], true);

    // Default params send no query. A list that could not be read is HTTP 200
    // with an error object and no products.
    assert!(captured[1].starts_with("GET /pinterest/products HTTP/1.1"), "{}", captured[1]);
    assert!(not_connected.get("products").is_none());
    assert_eq!(not_connected["error"]["code"], "pinterest_not_connected");

    assert!(
        captured[2].starts_with("GET /pinterest/products/validate?id=813744226420795884 HTTP/1.1"),
        "{}",
        captured[2]
    );
    assert_eq!(check["valid"], true);
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

// ─── Ads ─────────────────────────────────────────────────────────────────────

fn request_line(raw: &str) -> &str {
    raw.lines().next().unwrap_or_default()
}

fn request_body(raw: &str) -> serde_json::Value {
    let start = raw.find("\r\n\r\n").unwrap() + 4;
    serde_json::from_str(&raw[start..]).unwrap()
}

fn draft_params() -> AdDraftParams {
    AdDraftParams {
        name: Some("Autumn sale".into()),
        objective: "traffic".into(),
        platforms: vec!["meta".into()],
        budget: AdBudget { amount: 10.0, currency: "EUR".into(), ..Default::default() },
        schedule: Some(AdDraftSchedule { duration_days: Some(7) }),
        targeting: AdDraftTargeting {
            countries: vec!["NL".into()],
            automatic_audience: Some(true),
            ..Default::default()
        },
        ads: vec![AdDraftAd {
            media: vec![AdMediaFile {
                url: "https://example.com/sale.jpg".into(),
                r#type: "image".into(),
                ..Default::default()
            }],
            primary_text: "30% off".into(),
            link: "https://example.com".into(),
            ..Default::default()
        }],
    }
}

#[tokio::test]
async fn ads_draft_create_update_launch_and_activate_requests() {
    let ok = r#"{"data":{"group_id":"g-1"}}"#;
    let (base_url, handle) = spawn_stub(vec![
        http_response(201, "Created", &[], ok),
        http_response(200, "OK", &[], ok),
        http_response(200, "OK", &[], r#"{"data":{"group":{"group_id":"g-1"},"results":[{"campaign_id":"7","platform":"meta","ok":true}]}}"#),
        http_response(200, "OK", &[], r#"{"data":{"id":"7","ad_account_id":"12","status":"pending_review","createdAt":"2026-10-28T10:00:00.000Z"}}"#),
    ]);

    let client = client_for(&base_url, 0);
    let created = client.ads().drafts().create(draft_params()).await.unwrap();
    assert_eq!(created["data"]["group_id"], "g-1");
    client.ads().drafts().update("g-1", draft_params()).await.unwrap();
    let launch = client
        .ads()
        .drafts()
        .launch("g-1", LaunchAdDraftParams { confirm_budget_amount: Some(10.0), ..Default::default() })
        .await
        .unwrap();
    assert_eq!(launch["data"]["results"][0]["ok"], true);
    let activated = client.ads().activate("7", 25.0).await.unwrap();
    assert_eq!(activated["data"]["status"], "pending_review");
    // Ad ids arrive as strings and a campaign's timestamps in camelCase; the
    // id goes straight back into the next call.
    assert_eq!(activated["data"]["id"].as_str(), Some("7"));
    assert_eq!(activated["data"]["ad_account_id"], "12");
    assert_eq!(activated["data"]["createdAt"], "2026-10-28T10:00:00.000Z");

    let captured = handle.join().unwrap();
    let draft_body = serde_json::json!({
        "name": "Autumn sale",
        "objective": "traffic",
        "platforms": ["meta"],
        "budget": {"amount": 10.0, "currency": "EUR"},
        "schedule": {"duration_days": 7},
        "targeting": {"countries": ["NL"], "automatic_audience": true},
        "ads": [{
            "media": [{"url": "https://example.com/sale.jpg", "type": "image"}],
            "primary_text": "30% off",
            "link": "https://example.com"
        }]
    });
    assert_eq!(request_line(&captured[0]), "POST /ads/drafts HTTP/1.1");
    assert_eq!(request_body(&captured[0]), draft_body);
    assert_eq!(request_line(&captured[1]), "PUT /ads/drafts/g-1 HTTP/1.1");
    assert_eq!(request_body(&captured[1]), draft_body);
    assert_eq!(request_line(&captured[2]), "POST /ads/drafts/g-1/launch HTTP/1.1");
    assert_eq!(request_body(&captured[2]), serde_json::json!({"confirm_budget_amount": 10.0}));
    assert_eq!(request_line(&captured[3]), "POST /ads/campaigns/7/activate HTTP/1.1");
    assert_eq!(request_body(&captured[3]), serde_json::json!({"confirm_budget_amount": 25.0}));
}

// A call that can start or raise ad spend may have gone through when the API
// answers 5xx or drops the connection, so it is sent exactly once. Each stub
// below holds one more answer than the money call should use: a retry would
// take it and succeed, so the follow-up health check proves there was none.
#[tokio::test]
async fn ads_money_calls_are_not_retried_after_a_502() {
    let bad_gateway = || {
        http_response(502, "Bad Gateway", &[("Retry-After", "0")], r#"{"error":{"code":"bad_gateway","message":"upstream"}}"#)
    };
    let healthy = || http_response(200, "OK", &[], r#"{"status":"ok"}"#);
    let (base_url, handle) = spawn_stub(vec![
        bad_gateway(), healthy(),
        bad_gateway(), healthy(),
        bad_gateway(), healthy(),
        bad_gateway(), healthy(),
        bad_gateway(), healthy(),
    ]);
    let client = client_for(&base_url, 2);
    let ads = client.ads();

    let err = ads
        .drafts()
        .launch("g-1", LaunchAdDraftParams { confirm_budget_amount: Some(10.0), ..Default::default() })
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Server { status: 502, .. }), "{err:?}");
    client.health().await.unwrap();

    let err = ads.activate("7", 25.0).await.unwrap_err();
    assert!(matches!(err, Error::Server { status: 502, .. }), "{err:?}");
    client.health().await.unwrap();

    assert!(ads.activate_multi(&["7", "8"], 25.0).await.is_err());
    client.health().await.unwrap();
    assert!(ads.resume("7").await.is_err());
    client.health().await.unwrap();
    let raise = UpdateAdCampaignParams {
        budget: Some(AdBudget { amount: 40.0, currency: "EUR".into(), ..Default::default() }),
        confirm_budget_amount: Some(40.0),
        ..Default::default()
    };
    assert!(ads.update("7", raise).await.is_err());
    client.health().await.unwrap();

    let lines: Vec<String> = handle
        .join()
        .unwrap()
        .iter()
        .map(|raw| request_line(raw).to_owned())
        .collect();
    assert_eq!(
        lines,
        vec![
            "POST /ads/drafts/g-1/launch HTTP/1.1",
            "GET /health HTTP/1.1",
            "POST /ads/campaigns/7/activate HTTP/1.1",
            "GET /health HTTP/1.1",
            "POST /ads/campaigns/activate-multi HTTP/1.1",
            "GET /health HTTP/1.1",
            "POST /ads/campaigns/7/resume HTTP/1.1",
            "GET /health HTTP/1.1",
            "PATCH /ads/campaigns/7 HTTP/1.1",
            "GET /health HTTP/1.1",
        ]
    );
}

#[tokio::test]
async fn ads_money_call_is_not_retried_after_a_dropped_connection() {
    let (base_url, handle) = spawn_stub(vec![
        String::new(), // read the request, then close without an answer
        http_response(200, "OK", &[], r#"{"status":"ok"}"#),
    ]);
    let client = client_for(&base_url, 2);

    let err = client.ads().activate("7", 25.0).await.unwrap_err();
    assert!(matches!(err, Error::Connection(_)), "expected Connection, got {err:?}");
    client.health().await.unwrap();

    let captured = handle.join().unwrap();
    assert_eq!(request_line(&captured[0]), "POST /ads/campaigns/7/activate HTTP/1.1");
    assert_eq!(request_line(&captured[1]), "GET /health HTTP/1.1");
}

#[tokio::test]
async fn ads_money_call_is_retried_after_a_429() {
    let (base_url, handle) = spawn_stub(vec![
        http_response(429, "Too Many Requests", &[("Retry-After", "0")], r#"{"error":{"code":"rate_limit_exceeded","message":"slow down"}}"#),
        http_response(200, "OK", &[], r#"{"data":{"group":{"group_id":"g-1"},"results":[]}}"#),
    ]);
    let client = client_for(&base_url, 2);

    client
        .ads()
        .drafts()
        .launch("g-1", LaunchAdDraftParams { confirm_budget_amount: Some(10.0), ..Default::default() })
        .await
        .unwrap();
    let captured = handle.join().unwrap();
    assert_eq!(captured.len(), 2);
    assert_eq!(request_line(&captured[1]), "POST /ads/drafts/g-1/launch HTTP/1.1");
}

#[tokio::test]
async fn ads_read_is_retried_after_a_502() {
    let (base_url, handle) = spawn_stub(vec![
        http_response(502, "Bad Gateway", &[("Retry-After", "0")], r#"{"error":{"code":"bad_gateway","message":"upstream"}}"#),
        http_response(200, "OK", &[], r#"{"data":{"networks":[],"create_networks":["meta"]}}"#),
    ]);
    let client = client_for(&base_url, 1);

    let caps = client.ads().capabilities().await.unwrap();
    assert_eq!(caps["data"]["create_networks"][0], "meta");
    let captured = handle.join().unwrap();
    assert_eq!(captured.len(), 2);
    assert_eq!(request_line(&captured[0]), "GET /ads/capabilities HTTP/1.1");
    assert_eq!(request_line(&captured[1]), "GET /ads/capabilities HTTP/1.1");
}

#[tokio::test]
async fn ads_list_builds_the_query_and_encodes_ids() {
    let (base_url, handle) = spawn_stub(vec![
        http_response(200, "OK", &[], r#"{"data":[],"pagination":{"total":0}}"#),
        http_response(200, "OK", &[], r#"{"data":{"deleted":true}}"#),
    ]);
    let client = client_for(&base_url, 0);

    client
        .ads()
        .list(omnisocials::ListAdCampaignsParams {
            status: Some("active".into()),
            campaign_type: Some("boost".into()),
            limit: Some(10),
            ..Default::default()
        })
        .await
        .unwrap();
    let deleted = client.ads().drafts().delete("g 1/x").await.unwrap();
    assert_eq!(deleted["data"]["deleted"], true);

    let captured = handle.join().unwrap();
    assert_eq!(
        request_line(&captured[0]),
        "GET /ads/campaigns?status=active&campaign_type=boost&limit=10 HTTP/1.1"
    );
    assert_eq!(request_line(&captured[1]), "DELETE /ads/drafts/g%201%2Fx HTTP/1.1");
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
