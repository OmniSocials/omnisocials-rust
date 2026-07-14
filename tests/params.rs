//! Serialization tests for the typed param structs: `None` fields must be
//! dropped, unions must serialize untagged, and reserved names must rename.

use std::collections::HashMap;

use omnisocials::{
    CheckMediaParams, Content, CreatePostParams, MediaRefs, UpdateFolderParams, UpdateMediaParams,
    UpdatePostParams, UpdateWebhookParams, UserTag,
};
use serde_json::{json, Value};

fn keys(value: &Value) -> Vec<&str> {
    let mut keys: Vec<&str> = value.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    keys
}

#[test]
fn create_post_params_drop_none_fields() {
    let params = CreatePostParams {
        content: "Hello from Rust".into(),
        channels: Some(vec!["instagram".into(), "x".into()]),
        ..Default::default()
    };
    let value = serde_json::to_value(&params).unwrap();
    assert_eq!(keys(&value), vec!["channels", "content"]);
    assert_eq!(value["content"], "Hello from Rust");
    assert_eq!(value["channels"], json!(["instagram", "x"]));
}

#[test]
fn create_post_params_serialize_full_shape() {
    let params = CreatePostParams {
        content: Content::PerPlatform(HashMap::from([
            ("default".to_owned(), "Hello".to_owned()),
            ("x".to_owned(), "Hello X".to_owned()),
        ])),
        channels: Some(vec!["x".into()]),
        scheduled_at: Some("2026-08-01T09:00:00Z".into()),
        media_urls: Some(vec!["https://example.com/a.jpg"].into()),
        r#type: Some("story".into()),
        location_id: Some("1234".into()),
        user_tags: Some(vec![UserTag {
            username: "someone".into(),
            x: 0.5,
            y: 0.25,
            image_index: None,
        }]),
        x: Some(json!({
            "reply_settings": "following",
            "thread_parts": [
                { "text": "part one" },
                { "text": "part two", "media_urls": ["https://example.com/b.jpg"] }
            ]
        })),
        ..Default::default()
    };
    let value = serde_json::to_value(&params).unwrap();

    // `r#type` must serialize as "type".
    assert_eq!(value["type"], "story");
    // Per-platform content is a plain object (untagged).
    assert_eq!(value["content"]["default"], "Hello");
    assert_eq!(value["content"]["x"], "Hello X");
    // Shared media_urls is a plain array (untagged).
    assert_eq!(value["media_urls"], json!(["https://example.com/a.jpg"]));
    // UserTag drops its None image_index.
    assert_eq!(value["user_tags"][0], json!({"username": "someone", "x": 0.5, "y": 0.25}));
    // Platform options pass through verbatim.
    assert_eq!(value["x"]["thread_parts"][1]["text"], "part two");
    // Unset platform blocks are absent, not null.
    assert!(value.get("instagram").is_none());
    assert!(value.get("google_business").is_none());
}

#[test]
fn media_refs_per_platform_serializes_as_object() {
    let refs: MediaRefs =
        HashMap::from([("instagram".to_owned(), vec!["id1".to_owned(), "id2".to_owned()])]).into();
    assert_eq!(serde_json::to_value(&refs).unwrap(), json!({"instagram": ["id1", "id2"]}));
}

#[test]
fn update_post_params_default_is_empty_object() {
    let value = serde_json::to_value(UpdatePostParams::default()).unwrap();
    assert_eq!(value, json!({}));
}

#[test]
fn update_post_params_thread_parts_null_survives() {
    let params = UpdatePostParams {
        x: Some(json!({"thread_parts": null})),
        ..Default::default()
    };
    let value = serde_json::to_value(&params).unwrap();
    // The explicit null must survive so the API clears thread mode.
    assert!(value["x"].get("thread_parts").is_some());
    assert!(value["x"]["thread_parts"].is_null());
}

#[test]
fn update_media_params_double_option_folder_id() {
    // None => omitted entirely.
    let untouched = serde_json::to_value(UpdateMediaParams::default()).unwrap();
    assert_eq!(untouched, json!({}));

    // Some(None) => explicit JSON null (move to root).
    let to_root = serde_json::to_value(UpdateMediaParams {
        name: None,
        folder_id: Some(None),
    })
    .unwrap();
    assert_eq!(to_root, json!({"folder_id": null}));

    // Some(Some(id)) => the id.
    let to_folder = serde_json::to_value(UpdateMediaParams {
        name: Some("hero-v2".into()),
        folder_id: Some(Some("12".into())),
    })
    .unwrap();
    assert_eq!(to_folder, json!({"name": "hero-v2", "folder_id": "12"}));
}

#[test]
fn update_folder_params_double_option_parent_id() {
    let to_top = serde_json::to_value(UpdateFolderParams {
        name: None,
        parent_id: Some(None),
    })
    .unwrap();
    assert_eq!(to_top, json!({"parent_id": null}));
}

#[test]
fn check_media_params_drop_none() {
    let by_size = serde_json::to_value(CheckMediaParams {
        size_bytes: Some(300_000_000),
        mime: Some("video/quicktime".into()),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(by_size, json!({"size_bytes": 300_000_000u64, "mime": "video/quicktime"}));
}

#[test]
fn update_webhook_params_drop_none() {
    let value = serde_json::to_value(UpdateWebhookParams {
        is_active: Some(false),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(value, json!({"is_active": false}));
}
