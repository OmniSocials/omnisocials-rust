//! Webhook signature verification (Stripe-style scheme).
//!
//! Every OmniSocials webhook delivery carries an `X-OmniSocials-Signature`
//! header of the form `t=<unix>,v1=<hex>`, where the hex value is an
//! HMAC-SHA256 of `"{timestamp}.{rawBody}"` keyed with the webhook's signing
//! secret (returned once on create / rotate-secret).

use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;

use crate::error::Error;

type HmacSha256 = Hmac<Sha256>;

/// Default timestamp tolerance: 5 minutes.
pub const DEFAULT_TOLERANCE_SECS: u64 = 300;

/// Verify an OmniSocials webhook delivery and return the parsed event.
///
/// * `payload` is the RAW request body, exactly as received. Do not parse and
///   re-serialize it first: the signature is computed over the raw bytes.
/// * `signature` is the `X-OmniSocials-Signature` header value
///   (`t=<unix>,v1=<hex>`).
/// * `secret` is the webhook's signing secret.
/// * `tolerance_secs` is the max allowed age of the timestamp in seconds
///   (replay protection). Use [`DEFAULT_TOLERANCE_SECS`] (300) unless you
///   have a reason not to; `0` disables the staleness check.
///
/// The signature comparison is constant-time
/// ([`hmac::Mac::verify_slice`]). Returns [`Error::WebhookVerification`] on
/// any failure.
///
/// ```no_run
/// # fn handle(body: &[u8], header: &str) -> Result<(), omnisocials::Error> {
/// let event = omnisocials::webhooks::verify_signature(
///     body,
///     header,
///     "whsec_...",
///     omnisocials::webhooks::DEFAULT_TOLERANCE_SECS,
/// )?;
/// println!("{} for post {}", event["type"], event["data"]["post_id"]);
/// # Ok(())
/// # }
/// ```
pub fn verify_signature(
    payload: &[u8],
    signature: &str,
    secret: &str,
    tolerance_secs: u64,
) -> Result<Value, Error> {
    if secret.is_empty() {
        return Err(Error::WebhookVerification("No webhook secret provided.".into()));
    }
    if signature.is_empty() {
        return Err(Error::WebhookVerification(
            "No signature header provided. Expected the X-OmniSocials-Signature header value."
                .into(),
        ));
    }

    // Parse `t=<unix>,v1=<hex>` (tolerate extra/unknown pairs and multiple v1).
    let mut timestamp_raw: Option<&str> = None;
    let mut candidates: Vec<&str> = Vec::new();
    for part in signature.split(',') {
        if let Some((key, value)) = part.split_once('=') {
            match key.trim() {
                "t" => timestamp_raw = Some(value.trim()),
                "v1" => candidates.push(value.trim()),
                _ => {}
            }
        }
    }

    let timestamp_raw = match timestamp_raw {
        Some(raw) if raw.parse::<i64>().is_ok() => raw,
        _ => {
            return Err(Error::WebhookVerification(
                "Unable to extract timestamp from signature header. \
                 Expected format: t=<unix>,v1=<hex>."
                    .into(),
            ))
        }
    };
    if candidates.is_empty() {
        return Err(Error::WebhookVerification(
            "Unable to extract v1 signature from signature header. \
             Expected format: t=<unix>,v1=<hex>."
                .into(),
        ));
    }

    // HMAC-SHA256 over "{timestamp}.{rawBody}".
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| Error::WebhookVerification("Invalid webhook secret.".into()))?;
    mac.update(timestamp_raw.as_bytes());
    mac.update(b".");
    mac.update(payload);

    let matches = candidates.iter().any(|candidate| {
        match hex::decode(candidate) {
            // Constant-time comparison via hmac (subtle) internals.
            Ok(bytes) => mac.clone().verify_slice(&bytes).is_ok(),
            Err(_) => false,
        }
    });
    if !matches {
        return Err(Error::WebhookVerification(
            "Webhook signature verification failed: no v1 signature matches the expected signature."
                .into(),
        ));
    }

    // Replay protection: reject stale timestamps beyond the tolerance.
    let timestamp: i64 = timestamp_raw.parse().expect("validated above");
    if tolerance_secs > 0 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let age = now - timestamp;
        if age > tolerance_secs as i64 {
            return Err(Error::WebhookVerification(format!(
                "Webhook timestamp is outside the allowed tolerance of {tolerance_secs}s \
                 (event is {age}s old). Possible replay."
            )));
        }
    }

    serde_json::from_slice(payload).map_err(|_| {
        Error::WebhookVerification(
            "Webhook payload is not valid JSON (did you pass the raw request body?).".into(),
        )
    })
}
