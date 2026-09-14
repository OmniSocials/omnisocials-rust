use serde_json::Value;

use crate::client::{encode_path_segment, Client};
use crate::error::Error;
use crate::types::{GetMessagesParams, ListConversationsParams, NextUnansweredParams, ReplyParams};

/// `client.inbox()`: read and reply to social conversations - DMs, comments,
/// and mentions - across Instagram, Facebook, LinkedIn, TikTok, YouTube, X,
/// and Threads.
///
/// Threads conversations are `type` `"comment"` (replies people leave on the
/// user's Threads posts; conversation ids look like
/// `threads_comment_<rootPostId>`) and `"mention"`
/// (`threads_mention_<postId>`); there are no Threads DMs. The Threads inbox
/// needs a Threads connection with the reply permissions; connections made
/// before those permissions existed must be reconnected once.
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
    /// first), optionally filtered by `platform`, `type`, `unread`, or
    /// `unanswered` (only conversations that still need an answer; see
    /// [`ListConversationsParams::unanswered`]). Cursor-paginated: see the
    /// [`Inbox`] type docs for how to page.
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
        if let Some(unanswered) = params.unanswered {
            query.push(("unanswered", unanswered.to_string()));
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
    /// reply. The Threads inbox needs a Threads connection with the reply
    /// permission. When the connection lacks that permission (connected
    /// before it existed) this fails with [`Error::Auth`] (status 401) and
    /// code `reauth_required` (reconnect Threads to fix it).
    ///
    /// Replying to an X DM costs 2 prepaid credits per send, debited before
    /// the send and automatically refunded if the send fails. If the
    /// balance can't cover it, this fails with [`Error::Api`] (status 402)
    /// and code `insufficient_credits`. If the workspace's X inbox was
    /// auto-suspended for hitting a zero balance, it fails with code
    /// `x_inbox_suspended` instead; top up and re-enable the inbox to
    /// resume (DMs that arrived while suspended are not recovered).
    ///
    /// Set `include_next: Some(true)` to also get `next` (the next
    /// conversation that needs an answer, the same object [`next`](Self::next)
    /// returns under `data`, using its default queue order and filters;
    /// `null` when nothing is waiting) and `remaining` in the response. Saves
    /// the extra call when working through the inbox.
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
    /// comment someone left on one of your posts, on the platform, as the
    /// post owner (scope `inbox:write`). Facebook, Instagram, TikTok, YouTube
    /// and Threads comments (Threads: incoming top-level replies only;
    /// Threads does not allow hiding nested replies). On YouTube, hide sets
    /// the comment's moderation status to rejected, which removes it and its
    /// replies from public view; unhide publishes it again. The message keeps
    /// its place in the conversation and the response returns it under
    /// `data` with `hidden` flipped; a hidden comment no longer counts as
    /// unanswered. The account must have been connected with the moderation
    /// permission (Facebook `pages_manage_engagement`, Instagram
    /// `instagram_business_manage_comments`).
    ///
    /// Errors: 400 `unsupported_platform` (not an incoming comment on a
    /// supported platform), 400 `not_hideable` (Threads nested reply, or
    /// Threads refused), 401 `reauth_required` (the Threads reply permission
    /// or the TikTok comments authorization is missing or expired), 403
    /// `reconnect_required` (the account was connected without the
    /// comment-moderation permission; reconnect it in the dashboard), 404
    /// `not_found` (message not in this workspace) or
    /// `account_not_connected`, 429 `quota_exceeded` (YouTube's daily API
    /// quota is used up; retry after midnight Pacific), 502 `platform_error`
    /// (the platform rejected the call). The Threads inbox needs a Threads
    /// connection with the reply permissions; a connection made before those
    /// permissions existed answers 401 `reauth_required` until reconnected.
    pub async fn hide(&self, message_id: &str, hide: bool) -> Result<Value, Error> {
        self.client
            .post_json(
                &format!("/inbox/messages/{}/hide", encode_path_segment(message_id)),
                &serde_json::json!({ "hide": hide }),
            )
            .await
    }

    /// `DELETE /inbox/messages/:id` - delete a comment someone left on one of
    /// your posts, on the platform and from the inbox (scope `inbox:write`).
    /// Facebook, Instagram and TikTok comments only: YouTube's API does not
    /// let a channel delete other people's comments, hide those instead
    /// ([`hide`](Self::hide)). Replies under the deleted comment go with it
    /// (the platforms cascade the delete and the inbox mirrors that); their
    /// inbox ids come back as `removed_reply_ids`. A comment that is already
    /// gone on the platform is still removed from the inbox. This cannot be
    /// undone. The response is `{ "data": { "id", "conversation_id",
    /// "removed_reply_ids" } }`.
    ///
    /// Errors: 400 `unsupported_platform` (not an incoming Facebook,
    /// Instagram or TikTok comment), 401 `reauth_required` (the TikTok
    /// comments authorization expired), 403 `reconnect_required` (the account
    /// was connected without the comment-moderation permission; reconnect it
    /// in the dashboard), 404 `not_found` (message not in this workspace) or
    /// `account_not_connected`, 502 `platform_error` (the platform rejected
    /// the call).
    pub async fn delete_message(&self, message_id: &str) -> Result<Value, Error> {
        self.client
            .delete(&format!(
                "/inbox/messages/{}",
                encode_path_segment(message_id)
            ))
            .await
    }

    /// `GET /inbox/next` - the next conversation that needs an answer: a work
    /// queue for answering the inbox (scope `inbox:read`). Returns the oldest
    /// (by default) item that still needs a reply, together with its
    /// conversation so far and the post it belongs to, so a reply can be
    /// drafted from one call. An item needs an answer when it is the
    /// customer's latest DM with no reply after it (Instagram/Facebook DMs
    /// within the 24-hour messaging window only, since Meta refuses replies
    /// outside it), or a comment/mention that has not been replied to and is
    /// not hidden. Replies typed in the native apps count as answers (they
    /// are mirrored into the inbox), so a thread a colleague answered on
    /// their phone is not served again. Instagram mentions are skipped (no
    /// reply path). Looks at the last 30 days of activity.
    ///
    /// Only unread items are served by default: marking a conversation read
    /// ([`mark_read`](Self::mark_read)) is how to skip one for good; set
    /// `include_read: Some(true)` to include read-but-unanswered items.
    /// `exclude` is a session-local skip: conversation ids to leave out of
    /// this call (up to 100). `order` is `"oldest"` (default: the item that
    /// has waited longest first) or `"newest"`.
    ///
    /// The response is `{ "data": ..., "remaining": n }`. `data` is
    /// `{ "conversation", "message", "messages" }`, or `null` when nothing is
    /// waiting. `message` is the unanswered incoming item itself (the
    /// customer's latest DM, or the specific comment): its `id` is what
    /// [`hide`](Self::hide) and [`delete_message`](Self::delete_message)
    /// take, its `conversation_id` is what [`reply`](Self::reply) takes.
    /// `messages` is the conversation so far, oldest first (the most recent
    /// 50 messages for long DM threads). `remaining` is the number of
    /// unanswered items still waiting after this one (capped at 500), `0`
    /// when `data` is `null`. To chain the queue, set `include_next` on
    /// [`reply`](Self::reply) and it returns the next item in the same
    /// response. Errors: 400 `validation_error` (unknown platform, type or
    /// order).
    pub async fn next(&self, params: NextUnansweredParams) -> Result<Value, Error> {
        let mut query = Vec::new();
        if let Some(platform) = params.platform {
            query.push(("platform", platform));
        }
        if let Some(r#type) = params.r#type {
            query.push(("type", r#type));
        }
        if let Some(order) = params.order {
            query.push(("order", order));
        }
        if let Some(include_read) = params.include_read {
            query.push(("include_read", include_read.to_string()));
        }
        if let Some(exclude) = params.exclude.filter(|ids| !ids.is_empty()) {
            query.push(("exclude", exclude.join(",")));
        }
        self.client.get("/inbox/next", query).await
    }
}
