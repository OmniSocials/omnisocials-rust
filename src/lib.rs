//! # OmniSocials Rust SDK
//!
//! The official Rust client for the [OmniSocials API](https://docs.omnisocials.com):
//! schedule and publish social media posts to Instagram, Facebook, LinkedIn,
//! YouTube, TikTok, X, Pinterest, Bluesky, Threads, Mastodon, and Google
//! Business from one API.
//!
//! ```no_run
//! use omnisocials::{Client, CreatePostParams};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), omnisocials::Error> {
//!     let client = Client::from_env()?; // reads OMNISOCIALS_API_KEY
//!
//!     let post = client
//!         .posts()
//!         .create(CreatePostParams {
//!             content: "Hello from the SDK".into(),
//!             channels: Some(vec!["instagram".into(), "linkedin".into()]),
//!             scheduled_at: Some("2026-08-01T09:00:00Z".into()),
//!             ..Default::default()
//!         })
//!         .await?;
//!
//!     println!("created {}", post["data"]["id"]);
//!     Ok(())
//! }
//! ```
//!
//! All methods return the parsed response body as a [`serde_json::Value`],
//! as-is (single items as `{"data": {...}}`, lists as
//! `{"data": [...], "pagination": {...}}`); `204 No Content` returns
//! [`serde_json::Value::Null`]. Retries on 429 / 5xx / connection errors are
//! automatic, with exponential backoff and `Retry-After` support.
//!
//! Webhook deliveries are verified with
//! [`webhooks::verify_signature`].

#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod client;
mod error;
pub mod resources;
mod types;
pub mod webhooks;

pub use client::{Client, ClientBuilder, VERSION};
pub use error::Error;
pub use types::{
    AccountAnalyticsParams, AnalyticsOverviewParams, BestTimesParams, CheckMediaParams, Content,
    CreateFolderParams, CreatePostParams, CreateWebhookParams, GetMessagesParams,
    ListConversationsParams, ListMediaParams, ListPostsParams, MediaRefs, RecentPlatformPostsParams,
    ReplyParams, UpdateFolderParams, UpdateMediaParams, UpdatePostParams, UpdateWebhookParams,
    UploadMediaFromBase64Params, UploadMediaFromUrlParams, UploadMediaParams, UserTag,
};
