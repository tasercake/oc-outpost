use crate::bot::BotState;
use crate::types::error::Result;
use std::sync::Arc;
use teloxide::prelude::*;
use tracing::{debug, warn};

pub async fn dispatch_callback(bot: Bot, q: CallbackQuery, state: Arc<BotState>) -> Result<()> {
    let data = match q.data.as_deref() {
        Some(d) => d,
        None => {
            let _ = bot.answer_callback_query(q.id).await;
            return Ok(());
        }
    };

    debug!(callback_data = %data, "Dispatching callback query");

    if data.starts_with("perm:") {
        crate::bot::handlers::permissions::handle_permission_callback(bot, q, state).await
    } else if data.starts_with("close:") {
        crate::bot::handlers::close::handle_close_callback(bot, q, state).await
    } else {
        warn!(callback_data = %data, "Unknown callback prefix");
        let _ = bot.answer_callback_query(q.id).text("Unknown action").await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_perm_prefix_detected() {
        let data = "perm:sess:pid:allow";
        assert!(data.starts_with("perm:"));
    }

    #[test]
    fn test_close_prefix_detected() {
        let data = "close:123:confirm";
        assert!(data.starts_with("close:"));
    }

    #[test]
    fn test_unknown_prefix_detected() {
        let data = "unknown:data";
        assert!(!data.starts_with("perm:"));
        assert!(!data.starts_with("close:"));
    }
}
