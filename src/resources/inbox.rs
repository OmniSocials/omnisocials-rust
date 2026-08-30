use serde_json::Value;

use crate::client::{encode_path_segment, Client};
use crate::error::Error;
use crate::types::{GetMessagesParams, ListConversationsParams, ReplyParams};

/// `client.inbox()`: read and reply to social conversations - DMs, comments,
/// and mentions - across Instagram, Facebook, LinkedIn, TikTok, YouTube, X,
/// and Threads.
///
/// Threads conversations are `type` `"comment"` (replies people leave on the
/// user's Threads posts; conversation ids look like
/// `threads_comment_<rootPostId>`) and `"mention"`
/// (`threads_mention_<postId>`); there are no Threads DMs. The Threads inbox
/// is currently rolling out: until Meta approves the permissions it is
/// disabled on production and calls return a clear error, and it needs a
/// Threads connection with the reply permission.
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
    /// conversation. On Facebook and Instagram DMs, attach a single piece of
    /// media with `attachment_url` + `attachment_type` (`"image"`,
    /// `"video"`, `"audio"`, or `"file"`); `text` is optional when
    /// `attachment_url` is set (an attachment-only reply is allowed). Other
    /// platforms are text-only. The response returns the created message
    /// under `data`, including an `attachment` object on messages that
    /// carry media.
    ///
    /// On a Threads conversation the reply publishes as a native Threads
    /// reply. The Threads inbox is currently rolling out: until Meta approves
    /// the permissions it is disabled on production, and it needs a Threads
    /// connection with the reply permission. When the connection lacks that
    /// permission this fails with [`Error::Auth`] (status 401) and code
    /// `reauth_required` (reconnect Threads to fix it).
    ///
    /// Replying to an X DM costs 2 prepaid credits per send, debited before
    /// the send and automatically refunded if the send fails. If the
    /// balance can't cover it, this fails with [`Error::Api`] (status 402)
    /// and code `insufficient_credits`. If the workspace's X inbox was
    /// auto-suspended for hitting a zero balance, it fails with code
    /// `x_inbox_suspended` instead; top up and re-enable the inbox to
    /// resume (DMs that arrived while suspended are not recovered).
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

    /// `POST /inbox/messages/:id/hide` - hide (`true`) or unhide (`false`) a
    /// reply someone left on one of the user's Threads posts, as the post
    /// owner (scope `inbox:write`). Threads only for now, and only incoming
    /// top-level replies can be hidden (Threads does not allow hiding nested
    /// replies); the message keeps its place in the conversation. The
    /// response returns the updated message under `data` with `hidden`
    /// flipped.
    ///
    /// Errors: 400 `unsupported_platform` (not an incoming Threads reply, or
    /// the Threads inbox is not available yet), 400 `not_hideable` (nested
    /// reply or Threads refused), 401 `reauth_required` (the connection lacks
    /// the reply permission; reconnect Threads), 404 `not_found` (message not
    /// in this workspace) or `account_not_connected` (no Threads account).
    /// The Threads inbox is currently rolling out; until Meta approves the
    /// permissions it is disabled on production and calls return a clear
    /// error.
    pub async fn hide(&self, message_id: &str, hide: bool) -> Result<Value, Error> {
        self.client
            .post_json(
                &format!("/inbox/messages/{}/hide", encode_path_segment(message_id)),
                &serde_json::json!({ "hide": hide }),
            )
            .await
    }
}
