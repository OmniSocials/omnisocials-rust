//! Webhook signature verification round-trip tests.
//!
//! The `sign` helper mirrors the server's dispatcher exactly
//! (`backend/services/webhooks/webhookDispatcher.js`): the signed value is
//! `"{timestamp}.{rawBody}"`, HMAC-SHA256 hex digest, header
//! `t=<timestamp>,v1=<hex>`.

use hmac::{Hmac, Mac};
use omnisocials::webhooks::{verify_signature, DEFAULT_TOLERANCE_SECS};
use omnisocials::Error;
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

const SECRET: &str = "whsec_test_secret_for_round_trip";
const PAYLOAD: &str = r#"{"id":"evt_1","type":"post.published","data":{"post_id":42,"status":"published","targets":[{"platform":"instagram","status":"published","native_post_id":"179"}]}}"#;

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}

/// Sign exactly like the server does.
fn sign(secret: &str, timestamp: i64, raw_body: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(format!("{timestamp}.{raw_body}").as_bytes());
    let hex_digest = hex::encode(mac.finalize().into_bytes());
    format!("t={timestamp},v1={hex_digest}")
}

fn assert_verification_error(result: Result<serde_json::Value, Error>, needle: &str) {
    match result {
        Err(Error::WebhookVerification(message)) => {
            assert!(
                message.contains(needle),
                "expected message containing {needle:?}, got {message:?}"
            );
        }
        other => panic!("expected WebhookVerification error, got {other:?}"),
    }
}

#[test]
fn valid_signature_passes_and_returns_parsed_event() {
    let header = sign(SECRET, now(), PAYLOAD);
    let event =
        verify_signature(PAYLOAD.as_bytes(), &header, SECRET, DEFAULT_TOLERANCE_SECS).unwrap();

    assert_eq!(event["type"], "post.published");
    assert_eq!(event["data"]["post_id"], 42);
    assert_eq!(event["data"]["targets"][0]["platform"], "instagram");
}

#[test]
fn tampered_payload_fails() {
    let header = sign(SECRET, now(), PAYLOAD);
    let tampered = PAYLOAD.replace("post.published", "post.failed");
    assert_verification_error(
        verify_signature(tampered.as_bytes(), &header, SECRET, DEFAULT_TOLERANCE_SECS),
        "no v1 signature matches",
    );
}

#[test]
fn stale_timestamp_fails() {
    let stale = now() - (DEFAULT_TOLERANCE_SECS as i64) - 1;
    let header = sign(SECRET, stale, PAYLOAD);
    assert_verification_error(
        verify_signature(PAYLOAD.as_bytes(), &header, SECRET, DEFAULT_TOLERANCE_SECS),
        "outside the allowed tolerance",
    );

    // tolerance 0 disables the staleness check entirely.
    assert!(verify_signature(PAYLOAD.as_bytes(), &header, SECRET, 0).is_ok());
}

#[test]
fn wrong_secret_fails() {
    let header = sign("whsec_a_different_secret", now(), PAYLOAD);
    assert_verification_error(
        verify_signature(PAYLOAD.as_bytes(), &header, SECRET, DEFAULT_TOLERANCE_SECS),
        "no v1 signature matches",
    );
}

#[test]
fn malformed_headers_fail() {
    let cases: &[(&str, &str)] = &[
        ("", "No signature header"),
        ("complete garbage", "timestamp"),
        ("t=notanumber,v1=deadbeef", "timestamp"),
        ("v1=deadbeef", "timestamp"),
        (&format!("t={}", now()), "v1 signature"),
        (&format!("t={},v1=not-hex-at-all", now()), "no v1 signature matches"),
    ];
    for (header, needle) in cases {
        assert_verification_error(
            verify_signature(PAYLOAD.as_bytes(), header, SECRET, DEFAULT_TOLERANCE_SECS),
            needle,
        );
    }
}

#[test]
fn empty_secret_fails() {
    let header = sign(SECRET, now(), PAYLOAD);
    assert_verification_error(
        verify_signature(PAYLOAD.as_bytes(), &header, "", DEFAULT_TOLERANCE_SECS),
        "No webhook secret",
    );
}

#[test]
fn multiple_v1_candidates_pass_when_one_matches() {
    let timestamp = now();
    let valid = sign(SECRET, timestamp, PAYLOAD);
    let valid_hex = valid.split("v1=").nth(1).unwrap();
    let header = format!("t={timestamp},v1=deadbeef,v1={valid_hex},scheme=extra");
    assert!(
        verify_signature(PAYLOAD.as_bytes(), &header, SECRET, DEFAULT_TOLERANCE_SECS).is_ok()
    );
}

#[test]
fn non_json_payload_with_valid_signature_fails() {
    let body = "this is not json";
    let header = sign(SECRET, now(), body);
    assert_verification_error(
        verify_signature(body.as_bytes(), &header, SECRET, DEFAULT_TOLERANCE_SECS),
        "not valid JSON",
    );
}
