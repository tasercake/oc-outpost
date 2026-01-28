use crate::bot::{BotState, Command};
use crate::types::error::Result;
use std::sync::Arc;
use teloxide::prelude::*;

pub mod connect;
pub mod disconnect;
pub mod new;
pub mod sessions;

#[allow(unused_imports)]
pub use connect::handle_connect;
#[allow(unused_imports)]
pub use disconnect::handle_disconnect;
#[allow(unused_imports)]
pub use sessions::handle_sessions;

#[allow(dead_code)]
pub async fn handle_link(bot: Bot, msg: Message, cmd: Command, state: Arc<BotState>) -> Result<()> {
    let _ = (bot, msg, cmd, state);
    Ok(())
}

#[allow(dead_code)]
pub async fn handle_stream(
    bot: Bot,
    msg: Message,
    cmd: Command,
    state: Arc<BotState>,
) -> Result<()> {
    let _ = (bot, msg, cmd, state);
    Ok(())
}

#[allow(dead_code)]
pub async fn handle_session(
    bot: Bot,
    msg: Message,
    cmd: Command,
    state: Arc<BotState>,
) -> Result<()> {
    let _ = (bot, msg, cmd, state);
    Ok(())
}

#[allow(dead_code)]
pub async fn handle_status(
    bot: Bot,
    msg: Message,
    cmd: Command,
    state: Arc<BotState>,
) -> Result<()> {
    let _ = (bot, msg, cmd, state);
    Ok(())
}

#[allow(dead_code)]
pub async fn handle_clear(
    bot: Bot,
    msg: Message,
    cmd: Command,
    state: Arc<BotState>,
) -> Result<()> {
    let _ = (bot, msg, cmd, state);
    Ok(())
}

#[allow(dead_code)]
pub async fn handle_help(bot: Bot, msg: Message, cmd: Command, state: Arc<BotState>) -> Result<()> {
    let _ = (bot, msg, cmd, state);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_signatures_compile() {
        let _: fn(Bot, Message, Command, Arc<BotState>) -> _ = new::handle_new;
        let _: fn(Bot, Message, Command, Arc<BotState>) -> _ = handle_sessions;
        let _: fn(Bot, Message, Command, Arc<BotState>) -> _ = handle_connect;
        let _: fn(Bot, Message, Command, Arc<BotState>) -> _ = handle_disconnect;
        let _: fn(Bot, Message, Command, Arc<BotState>) -> _ = handle_link;
        let _: fn(Bot, Message, Command, Arc<BotState>) -> _ = handle_stream;
        let _: fn(Bot, Message, Command, Arc<BotState>) -> _ = handle_session;
        let _: fn(Bot, Message, Command, Arc<BotState>) -> _ = handle_status;
        let _: fn(Bot, Message, Command, Arc<BotState>) -> _ = handle_clear;
        let _: fn(Bot, Message, Command, Arc<BotState>) -> _ = handle_help;
    }
}
