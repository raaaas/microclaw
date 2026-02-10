use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use teloxide::prelude::*;
use teloxide::types::InputFile;

use super::{authorize_chat_access, schema_object, Tool, ToolResult};
use crate::channel::{deliver_and_store_bot_message, enforce_channel_policy};
use crate::claude::ToolDefinition;
use crate::db::{call_blocking, Database, StoredMessage};

pub struct SendMessageTool {
    bot: Bot,
    db: Arc<Database>,
    bot_username: String,
}

impl SendMessageTool {
    pub fn new(bot: Bot, db: Arc<Database>, bot_username: String) -> Self {
        SendMessageTool {
            bot,
            db,
            bot_username,
        }
    }
}

#[async_trait]
impl Tool for SendMessageTool {
    fn name(&self) -> &str {
        "send_message"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "send_message".into(),
            description: "Send a message mid-conversation. For Telegram chats it sends via Telegram; for local web chats it appends to the web conversation.".into(),
            input_schema: schema_object(
                json!({
                    "chat_id": {
                        "type": "integer",
                        "description": "The target chat ID"
                    },
                    "text": {
                        "type": "string",
                        "description": "The message text to send"
                    },
                    "attachment_path": {
                        "type": "string",
                        "description": "Optional local file path to send as Telegram document"
                    },
                    "caption": {
                        "type": "string",
                        "description": "Optional caption used when sending attachment"
                    }
                }),
                &["chat_id"],
            ),
        }
    }

    async fn execute(&self, input: serde_json::Value) -> ToolResult {
        let chat_id = match input.get("chat_id").and_then(|v| v.as_i64()) {
            Some(id) => id,
            None => return ToolResult::error("Missing required parameter: chat_id".into()),
        };
        let text = input
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let attachment_path = input
            .get("attachment_path")
            .and_then(|v| v.as_str())
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let caption = input
            .get("caption")
            .and_then(|v| v.as_str())
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());

        if text.is_empty() && attachment_path.is_none() {
            return ToolResult::error("Provide text and/or attachment_path".into());
        }

        if let Err(e) = authorize_chat_access(&input, chat_id) {
            return ToolResult::error(e);
        }

        if let Err(e) = enforce_channel_policy(self.db.clone(), &input, chat_id).await {
            return ToolResult::error(e);
        }

        if let Some(path) = attachment_path {
            let chat_type =
                match call_blocking(self.db.clone(), move |db| db.get_chat_type(chat_id)).await {
                    Ok(v) => v,
                    Err(e) => return ToolResult::error(format!("Failed to read chat type: {e}")),
                };

            let is_telegram = matches!(
                chat_type.as_deref(),
                Some("telegram_private")
                    | Some("telegram_group")
                    | Some("telegram_supergroup")
                    | Some("telegram_channel")
                    | Some("private")
                    | Some("group")
                    | Some("supergroup")
                    | Some("channel")
            );

            if !is_telegram {
                return ToolResult::error(
                    "attachment sending is currently supported for Telegram chats only".into(),
                );
            }

            let file_path = PathBuf::from(&path);
            if !file_path.is_file() {
                return ToolResult::error(format!(
                    "attachment_path not found or not a file: {path}"
                ));
            }

            let used_caption = caption.or_else(|| {
                if text.is_empty() {
                    None
                } else {
                    Some(text.clone())
                }
            });

            let mut req = self
                .bot
                .send_document(ChatId(chat_id), InputFile::file(file_path.clone()));
            if let Some(c) = &used_caption {
                req = req.caption(c.clone());
            }

            if let Err(e) = req.await {
                return ToolResult::error(format!("Failed to send attachment: {e}"));
            }

            let content = match used_caption {
                Some(c) => format!("[attachment:{}] {}", file_path.display(), c),
                None => format!("[attachment:{}]", file_path.display()),
            };
            let bot_name = self.bot_username.clone();
            let msg = StoredMessage {
                id: uuid::Uuid::new_v4().to_string(),
                chat_id,
                sender_name: bot_name,
                content,
                is_from_bot: true,
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            if let Err(e) = call_blocking(self.db.clone(), move |db| db.store_message(&msg)).await {
                return ToolResult::error(format!(
                    "Attachment sent but failed to store message: {e}"
                ));
            }

            ToolResult::success("Attachment sent successfully.".into())
        } else {
            match deliver_and_store_bot_message(
                &self.bot,
                self.db.clone(),
                &self.bot_username,
                chat_id,
                &text,
            )
            .await
            {
                Ok(_) => ToolResult::success("Message sent successfully.".into()),
                Err(e) => ToolResult::error(e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn test_db() -> (Arc<Database>, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("microclaw_sendmsg_{}", uuid::Uuid::new_v4()));
        let db = Arc::new(Database::new(dir.to_str().unwrap()).unwrap());
        (db, dir)
    }

    fn cleanup(dir: &std::path::Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn test_send_message_permission_denied_before_network() {
        let (db, dir) = test_db();
        let tool = SendMessageTool::new(Bot::new("123456:TEST_TOKEN"), db, "bot".into());
        let result = tool
            .execute(json!({
                "chat_id": 200,
                "text": "hello",
                "__microclaw_auth": {
                    "caller_chat_id": 100,
                    "control_chat_ids": []
                }
            }))
            .await;
        assert!(result.is_error);
        assert!(result.content.contains("Permission denied"));
        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_send_message_web_target_writes_to_db() {
        let (db, dir) = test_db();
        db.upsert_chat(999, Some("web-main"), "web").unwrap();

        let tool = SendMessageTool::new(Bot::new("123456:TEST_TOKEN"), db.clone(), "bot".into());
        let result = tool
            .execute(json!({
                "chat_id": 999,
                "text": "hello web",
                "__microclaw_auth": {
                    "caller_chat_id": 999,
                    "control_chat_ids": []
                }
            }))
            .await;
        assert!(!result.is_error, "{}", result.content);

        let all = db.get_all_messages(999).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].content, "hello web");
        assert!(all[0].is_from_bot);
        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_send_message_web_caller_cross_chat_denied() {
        let (db, dir) = test_db();
        db.upsert_chat(100, Some("web-main"), "web").unwrap();
        db.upsert_chat(200, Some("tg"), "private").unwrap();

        let tool = SendMessageTool::new(Bot::new("123456:TEST_TOKEN"), db, "bot".into());
        let result = tool
            .execute(json!({
                "chat_id": 200,
                "text": "hello",
                "__microclaw_auth": {
                    "caller_chat_id": 100,
                    "control_chat_ids": [100]
                }
            }))
            .await;
        assert!(result.is_error);
        assert!(result
            .content
            .contains("web chats cannot operate on other chats"));
        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_send_message_requires_text_or_attachment() {
        let (db, dir) = test_db();
        let tool = SendMessageTool::new(Bot::new("123456:TEST_TOKEN"), db, "bot".into());
        let result = tool
            .execute(json!({
                "chat_id": 999,
                "text": "   "
            }))
            .await;
        assert!(result.is_error);
        assert!(result
            .content
            .contains("Provide text and/or attachment_path"));
        cleanup(&dir);
    }

    #[tokio::test]
    async fn test_send_attachment_non_telegram_rejected_without_network() {
        let (db, dir) = test_db();
        db.upsert_chat(999, Some("web-main"), "web").unwrap();

        let attachment = dir.join("sample.txt");
        std::fs::write(&attachment, "hello").unwrap();

        let tool = SendMessageTool::new(Bot::new("123456:TEST_TOKEN"), db, "bot".into());
        let result = tool
            .execute(json!({
                "chat_id": 999,
                "attachment_path": attachment.to_string_lossy(),
                "caption": "test"
            }))
            .await;
        assert!(result.is_error);
        assert!(result
            .content
            .contains("currently supported for Telegram chats only"));
        cleanup(&dir);
    }
}
