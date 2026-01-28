#[allow(unused_imports)]
use crate::bot::{BotState, Command};
#[allow(unused_imports)]
use std::sync::Arc;
#[allow(unused_imports)]
use teloxide::prelude::*;

pub mod clear;
pub mod connect;
pub mod disconnect;
pub mod help;
pub mod link;
pub mod new;
pub mod permissions;
pub mod session;
pub mod sessions;
pub mod status;
pub mod stream;

#[allow(unused_imports)]
pub use clear::handle_clear;
#[allow(unused_imports)]
pub use connect::handle_connect;
#[allow(unused_imports)]
pub use disconnect::handle_disconnect;
#[allow(unused_imports)]
pub use help::handle_help;
#[allow(unused_imports)]
pub use link::handle_link;
#[allow(unused_imports)]
pub use new::handle_new;
#[allow(unused_imports)]
pub use permissions::{handle_permission_callback, handle_permission_request};
#[allow(unused_imports)]
pub use session::handle_session;
#[allow(unused_imports)]
pub use sessions::handle_sessions;
#[allow(unused_imports)]
pub use status::handle_status;
#[allow(unused_imports)]
pub use stream::handle_stream;

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
        let _: fn(Bot, Message, Command, Arc<BotState>) -> _ = help::handle_help;
    }

    #[test]
    fn test_status_handler_signature() {
        let _: fn(Bot, Message, Command, Arc<BotState>) -> _ = status::handle_status;
    }
}
