use serde_json::Value;

use crate::client::{encode_path_segment, Client};
use crate::error::Error;
use crate::types::{GetMessagesParams, ListConversationsParams, ReplyParams};

/// `client.inbox()`: read and reply to social conversations - DMs, comments,
/// and mentions - across Instagram, Facebook, and LinkedIn.
///
/// The two list endpoints ([`list_conversations`](Inbox::list_conversations)
/// and [`get_messages`](Inbox::get_messages)) use **cursor** pagination
/// rather than the offset pagination used elsewhere in the API. Each response
/// carries a `pagination` object shaped
/// `{ "next_cursor": String | null, "has_more": bool, "limit": u32 }`. To walk
/// pages, pass a non-null `next_cursor` back as the `cursor` parameter and stop
/// once `has_more` is `false`.
///
/// Conversation ids can contain reserved characters (LinkedIn ids look like
/// `linkedin_comment_urn:li:activity:123`); every method percent-encodes the
/// id into the path for you, so pass it exactly as the API returned it.
#[derive(Debug, Clone, Copy)]
pub struct Inbox<'a> {
    pub(crate) client: &'a Client,
}

impl Inbox<'_> {
    /// `GET /inbox/conversations` - list conversations (most recent activity
    /// first), optionally filtered by `platform`, `type`, or `unread`.
    /// Cursor-paginated: see the [`Inbox`] type docs for how to page.
    pub async fn list_conversations(
        &self,
        params: ListConversationsParams,
    ) -> Result<Value, Error> {
        let mut query = Vec::new();
        if let Some(platform) = params.platform {
            query.push(("platform", platform));
        }
        if let Some(r#type) = params.r#type {
            query.push(("type", r#type));
        }
        if let Some(unread) = params.unread {
            query.push(("unread", unread.to_string()));
        }
        if let Some(limit) = params.limit {
            query.push(("limit", limit.to_string()));
        }
        if let Some(cursor) = params.cursor {
            query.push(("cursor", cursor));
        }
        self.client.get("/inbox/conversations", query).await
    }

    /// `GET /inbox/conversations/:id/messages` - the full message history for
    /// one conversation (most recent first). Cursor-paginated: see the
    /// [`Inbox`] type docs for how to page.
    pub async fn get_messages(
        &self,
        conversation_id: &str,
        params: GetMessagesParams,
    ) -> Result<Value, Error> {
        let mut query = Vec::new();
        if let Some(limit) = params.limit {
            query.push(("limit", limit.to_string()));
        }
        if let Some(cursor) = params.cursor {
            query.push(("cursor", cursor));
        }
        self.client
            .get(
                &format!(
                    "/inbox/conversations/{}/messages",
                    encode_path_segment(conversation_id)
                ),
                query,
            )
            .await
    }

    /// `POST /inbox/conversations/:id/read` - mark every message in the
    /// conversation as read. The response reports `conversation_id` and how
    /// many messages were `marked_read`.
    pub async fn mark_read(&self, conversation_id: &str) -> Result<Value, Error> {
        self.client
            .post_empty(&format!(
                "/inbox/conversations/{}/read",
                encode_path_segment(conversation_id)
            ))
            .await
    }

    /// `POST /inbox/conversations/:id/reply` - send a reply in the
    /// conversation. `text` is required; attach a single piece of media with
    /// `attachment_url` + `attachment_type` (`"image"`, `"video"`, `"audio"`,
    /// or `"file"`). The response returns the created message under `data`.
    pub async fn reply(
        &self,
        conversation_id: &str,
        params: ReplyParams,
    ) -> Result<Value, Error> {
        self.client
            .post_json(
                &format!(
                    "/inbox/conversations/{}/reply",
                    encode_path_segment(conversation_id)
                ),
                &params,
            )
            .await
    }
}
