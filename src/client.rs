//! The [`Client`] and its core request loop.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::header::{ACCEPT, AUTHORIZATION, RETRY_AFTER};
use reqwest::Method;
use serde::Serialize;
use serde_json::Value;

use crate::error::Error;
use crate::resources::{
    Accounts, Analytics, Audio, Folders, Inbox, Locations, Media, Posts, Webhooks,
};

/// Crate version, used in the `User-Agent` header.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_BASE_URL: &str = "https://api.omnisocials.com/v1";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_MAX_RETRIES: u32 = 2;
const ENV_API_KEY: &str = "OMNISOCIALS_API_KEY";

/// Async client for the OmniSocials API.
///
/// ```no_run
/// # async fn run() -> Result<(), omnisocials::Error> {
/// let client = omnisocials::Client::from_env()?; // reads OMNISOCIALS_API_KEY
/// let posts = client.posts().list(Default::default()).await?;
/// # Ok(())
/// # }
/// ```
///
/// Cloning is cheap (the underlying HTTP connection pool is shared).
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
    timeout: Duration,
    max_retries: u32,
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.base_url)
            .field("timeout", &self.timeout)
            .field("max_retries", &self.max_retries)
            .field("api_key", &"omsk_***redacted***")
            .finish()
    }
}

/// Builder for [`Client`]. Created via [`Client::builder`].
#[derive(Debug, Default, Clone)]
pub struct ClientBuilder {
    api_key: Option<String>,
    base_url: Option<String>,
    timeout: Option<Duration>,
    max_retries: Option<u32>,
}

impl ClientBuilder {
    /// API key (`omsk_live_*` / `omsk_test_*`). Wins over the
    /// `OMNISOCIALS_API_KEY` environment variable.
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// API base URL. Defaults to `https://api.omnisocials.com/v1`.
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Per-request timeout. Defaults to 30 seconds.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Automatic retries on 429 / 5xx / connection errors. Defaults to 2.
    pub fn max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = Some(max_retries);
        self
    }

    /// Build the [`Client`].
    ///
    /// Returns [`Error::Auth`] (code `"missing_api_key"`) when no API key was
    /// provided and `OMNISOCIALS_API_KEY` is not set.
    pub fn build(self) -> Result<Client, Error> {
        let api_key = self
            .api_key
            .filter(|key| !key.is_empty())
            .or_else(|| std::env::var(ENV_API_KEY).ok().filter(|key| !key.is_empty()))
            .ok_or_else(|| Error::Auth {
                status: 401,
                code: Some("missing_api_key".into()),
                message: format!(
                    "No API key provided. Pass one with Client::builder().api_key(\"omsk_live_...\") \
                     or set the {ENV_API_KEY} environment variable. Create a key in the \
                     OmniSocials app under Settings -> API Keys."
                ),
                body: None,
            })?;

        let timeout = self.timeout.unwrap_or(DEFAULT_TIMEOUT);
        let base_url = self
            .base_url
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_owned())
            .trim_end_matches('/')
            .to_owned();

        let http = reqwest::Client::builder()
            .timeout(timeout)
            .user_agent(format!("omnisocials-rust/{VERSION}"))
            .build()?;

        Ok(Client {
            http,
            api_key,
            base_url,
            timeout,
            max_retries: self.max_retries.unwrap_or(DEFAULT_MAX_RETRIES),
        })
    }
}

/// Request body shapes the core loop knows how to (re)build per attempt.
pub(crate) enum RequestBody {
    Empty,
    Json(Value),
    Multipart {
        file: Vec<u8>,
        filename: String,
        fields: Vec<(&'static str, String)>,
    },
}

impl Client {
    /// Build a client from the `OMNISOCIALS_API_KEY` environment variable.
    pub fn from_env() -> Result<Client, Error> {
        Client::builder().build()
    }

    /// Build a client with an explicit API key and default settings.
    pub fn new(api_key: impl Into<String>) -> Result<Client, Error> {
        Client::builder().api_key(api_key).build()
    }

    /// Start configuring a client.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// The configured base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// The configured per-request timeout.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// The configured retry budget.
    pub fn max_retries(&self) -> u32 {
        self.max_retries
    }

    // ─── Resource accessors ──────────────────────────────────────────────────

    /// Posts: list, get, create, create_and_publish, update, delete, publish,
    /// recent_platform.
    pub fn posts(&self) -> Posts<'_> {
        Posts { client: self }
    }

    /// Media Library: list, get, upload, upload_from_url, upload_from_base64,
    /// create_upload_url, check, update, delete.
    pub fn media(&self) -> Media<'_> {
        Media { client: self }
    }

    /// Media folders: list, create, update, delete.
    pub fn folders(&self) -> Folders<'_> {
        Folders { client: self }
    }

    /// Connected social accounts: list, get.
    pub fn accounts(&self) -> Accounts<'_> {
        Accounts { client: self }
    }

    /// Analytics: post, posts, overview, accounts, best_times.
    pub fn analytics(&self) -> Analytics<'_> {
        Analytics { client: self }
    }

    /// Instagram location tagging: search, validate.
    pub fn locations(&self) -> Locations<'_> {
        Locations { client: self }
    }

    /// Instagram Reels licensed audio (Meta catalog): search.
    pub fn audio(&self) -> Audio<'_> {
        Audio { client: self }
    }

    /// Webhook endpoint management: list, get, create, update, delete,
    /// rotate_secret. (Signature verification lives in
    /// [`crate::webhooks::verify_signature`].)
    pub fn webhooks(&self) -> Webhooks<'_> {
        Webhooks { client: self }
    }

    /// Social Inbox: list_conversations, get_messages, mark_read, reply.
    pub fn inbox(&self) -> Inbox<'_> {
        Inbox { client: self }
    }

    /// `GET /health` (no scopes required).
    pub async fn health(&self) -> Result<Value, Error> {
        self.get("/health", Vec::new()).await
    }

    // ─── HTTP verb helpers used by resources ─────────────────────────────────

    pub(crate) async fn get(
        &self,
        path: &str,
        query: Vec<(&'static str, String)>,
    ) -> Result<Value, Error> {
        self.request(Method::GET, path, query, RequestBody::Empty).await
    }

    pub(crate) async fn post_json<T: Serialize>(&self, path: &str, body: &T) -> Result<Value, Error> {
        let value = serde_json::to_value(body).map_err(|err| Error::Validation {
            status: 400,
            code: Some("serialization_error".into()),
            message: format!("Could not serialize the request body: {err}"),
            body: None,
        })?;
        self.request(Method::POST, path, Vec::new(), RequestBody::Json(value)).await
    }

    pub(crate) async fn post_empty(&self, path: &str) -> Result<Value, Error> {
        self.request(Method::POST, path, Vec::new(), RequestBody::Empty).await
    }

    pub(crate) async fn post_multipart(
        &self,
        path: &str,
        file: Vec<u8>,
        filename: String,
        fields: Vec<(&'static str, String)>,
    ) -> Result<Value, Error> {
        self.request(
            Method::POST,
            path,
            Vec::new(),
            RequestBody::Multipart { file, filename, fields },
        )
        .await
    }

    pub(crate) async fn patch_json<T: Serialize>(&self, path: &str, body: &T) -> Result<Value, Error> {
        let value = serde_json::to_value(body).map_err(|err| Error::Validation {
            status: 400,
            code: Some("serialization_error".into()),
            message: format!("Could not serialize the request body: {err}"),
            body: None,
        })?;
        self.request(Method::PATCH, path, Vec::new(), RequestBody::Json(value)).await
    }

    pub(crate) async fn delete(&self, path: &str) -> Result<Value, Error> {
        self.request(Method::DELETE, path, Vec::new(), RequestBody::Empty).await
    }

    /// Core request loop: sends the request and retries 429 / 5xx /
    /// connection errors with exponential backoff + jitter (honoring
    /// `Retry-After` when present). Returns the parsed response body as-is;
    /// `204 No Content` returns [`Value::Null`].
    async fn request(
        &self,
        method: Method,
        path: &str,
        query: Vec<(&'static str, String)>,
        body: RequestBody,
    ) -> Result<Value, Error> {
        let url = format!(
            "{}{}{}",
            self.base_url,
            if path.starts_with('/') { "" } else { "/" },
            path
        );

        let mut attempt: u32 = 0;
        loop {
            let mut request = self
                .http
                .request(method.clone(), &url)
                .header(AUTHORIZATION, format!("Bearer {}", self.api_key))
                .header(ACCEPT, "application/json");

            if !query.is_empty() {
                request = request.query(&query);
            }

            match &body {
                RequestBody::Empty => {}
                RequestBody::Json(value) => {
                    request = request.json(value);
                }
                RequestBody::Multipart { file, filename, fields } => {
                    // multipart::Form is consumed on send, so rebuild it per attempt.
                    let mut form = reqwest::multipart::Form::new();
                    for (key, value) in fields {
                        form = form.text(*key, value.clone());
                    }
                    let part =
                        reqwest::multipart::Part::bytes(file.clone()).file_name(filename.clone());
                    form = form.part("file", part);
                    request = request.multipart(form);
                }
            }

            // Connection errors (incl. timeouts): retryable.
            let response = match request.send().await {
                Ok(response) => response,
                Err(err) => {
                    if attempt < self.max_retries {
                        sleep(self.backoff(attempt, None)).await;
                        attempt += 1;
                        continue;
                    }
                    return Err(Error::Connection(err));
                }
            };

            let status = response.status().as_u16();

            if (200..300).contains(&status) {
                if status == 204 {
                    return Ok(Value::Null);
                }
                let text = response.text().await.map_err(Error::Connection)?;
                if text.is_empty() {
                    return Ok(Value::Null);
                }
                // Non-JSON 2xx bodies are passed through as a JSON string.
                return Ok(serde_json::from_str(&text).unwrap_or(Value::String(text)));
            }

            let retry_after = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(parse_retry_after);

            let retryable = status == 429 || status >= 500;
            if retryable && attempt < self.max_retries {
                sleep(self.backoff(attempt, retry_after)).await;
                attempt += 1;
                continue;
            }

            let text = response.text().await.unwrap_or_default();
            return Err(Error::from_response(status, &text, retry_after));
        }
    }

    /// Exponential backoff (0.5s, 1s, 2s, ...) with 75%-125% jitter, capped
    /// at 30s; a `Retry-After` value wins (capped at 60s).
    fn backoff(&self, attempt: u32, retry_after: Option<f64>) -> Duration {
        if let Some(seconds) = retry_after {
            return Duration::from_secs_f64(seconds.clamp(0.0, 60.0));
        }
        let base = 0.5 * 2f64.powi(attempt.min(16) as i32);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let jitter = 0.75 + (nanos as f64 / 1_000_000_000.0) * 0.5;
        Duration::from_secs_f64((base * jitter).min(30.0))
    }
}

/// Percent-encode a path segment (RFC 3986 unreserved characters pass through).
pub(crate) fn encode_path_segment(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for byte in segment.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Parse a `Retry-After` header value: delta-seconds or an HTTP-date.
fn parse_retry_after(value: &str) -> Option<f64> {
    let value = value.trim();
    if let Ok(seconds) = value.parse::<f64>() {
        return if seconds.is_finite() && seconds >= 0.0 { Some(seconds) } else { None };
    }
    let target = parse_http_date(value)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;
    Some((target - now).max(0) as f64)
}

/// Parse an RFC 7231 IMF-fixdate ("Sun, 06 Nov 1994 08:49:37 GMT") to a unix
/// timestamp. Returns `None` for anything else.
fn parse_http_date(value: &str) -> Option<i64> {
    // Strip the "Sun, " weekday prefix if present.
    let rest = match value.split_once(", ") {
        Some((_weekday, rest)) => rest,
        None => value,
    };
    let mut parts = rest.split_whitespace();
    let day: u32 = parts.next()?.parse().ok()?;
    let month = match parts.next()? {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return None,
    };
    let year: i64 = parts.next()?.parse().ok()?;
    let mut time = parts.next()?.split(':');
    let hour: i64 = time.next()?.parse().ok()?;
    let minute: i64 = time.next()?.parse().ok()?;
    let second: i64 = time.next()?.parse().ok()?;
    if !(1..=31).contains(&day) || hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    Some(days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + second)
}

/// Days since 1970-01-01 for a proleptic Gregorian date
/// (Howard Hinnant's days_from_civil algorithm).
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

// ─── Runtime-agnostic async sleep ────────────────────────────────────────────
//
// The crate keeps `tokio` out of its runtime dependencies, so retry backoff
// uses a small thread-backed timer future that works on any executor. Retries
// are rare, so one short-lived thread per backoff is a fine trade.

pub(crate) fn sleep(duration: Duration) -> ThreadSleep {
    ThreadSleep {
        duration,
        state: Arc::new(Mutex::new(SleepState {
            completed: duration.is_zero(),
            spawned: false,
            waker: None,
        })),
    }
}

struct SleepState {
    completed: bool,
    spawned: bool,
    waker: Option<Waker>,
}

pub(crate) struct ThreadSleep {
    duration: Duration,
    state: Arc<Mutex<SleepState>>,
}

impl Future for ThreadSleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut state = self.state.lock().expect("sleep state poisoned");
        if state.completed {
            return Poll::Ready(());
        }
        state.waker = Some(cx.waker().clone());
        if !state.spawned {
            state.spawned = true;
            let shared = Arc::clone(&self.state);
            let duration = self.duration;
            std::thread::spawn(move || {
                std::thread::sleep(duration);
                let mut state = shared.lock().expect("sleep state poisoned");
                state.completed = true;
                if let Some(waker) = state.waker.take() {
                    waker.wake();
                }
            });
        }
        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_numeric_retry_after() {
        assert_eq!(parse_retry_after("7"), Some(7.0));
        assert_eq!(parse_retry_after(" 0.5 "), Some(0.5));
        assert_eq!(parse_retry_after("-3"), None);
        assert_eq!(parse_retry_after("soon"), None);
    }

    #[test]
    fn parses_http_date_retry_after() {
        // Known fixture: Sun, 06 Nov 1994 08:49:37 GMT == 784111777.
        assert_eq!(parse_http_date("Sun, 06 Nov 1994 08:49:37 GMT"), Some(784_111_777));
        // Epoch itself.
        assert_eq!(parse_http_date("Thu, 01 Jan 1970 00:00:00 GMT"), Some(0));
        // A past date clamps to 0 seconds.
        assert_eq!(parse_retry_after("Sun, 06 Nov 1994 08:49:37 GMT"), Some(0.0));
        assert_eq!(parse_http_date("not a date"), None);
    }

    #[test]
    fn encodes_path_segments() {
        assert_eq!(encode_path_segment("abc-123_ok.~"), "abc-123_ok.~");
        assert_eq!(encode_path_segment("a/b c?"), "a%2Fb%20c%3F");
    }

    #[test]
    fn thread_sleep_completes() {
        block_on(sleep(Duration::from_millis(30)));
    }

    /// Minimal single-future block_on so the sleep test needs no runtime.
    fn block_on<F: Future>(future: F) -> F::Output {
        use std::sync::mpsc;
        use std::task::Wake;

        struct ChannelWaker(mpsc::Sender<()>);
        impl Wake for ChannelWaker {
            fn wake(self: Arc<Self>) {
                let _ = self.0.send(());
            }
        }

        let (tx, rx) = mpsc::channel::<()>();
        let waker = Waker::from(Arc::new(ChannelWaker(tx)));
        let mut cx = Context::from_waker(&waker);
        let mut future = Box::pin(future);
        loop {
            match future.as_mut().poll(&mut cx) {
                Poll::Ready(output) => return output,
                Poll::Pending => {
                    rx.recv().expect("waker dropped without waking");
                }
            }
        }
    }
}
