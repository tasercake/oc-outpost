mod api;
mod bot;
mod config;
mod db;
mod forum;
mod integration;
mod opencode;
mod orchestrator;
mod telegram;
mod types;

use anyhow::Result;
use bot::{
    handle_clear, handle_connect, handle_disconnect, handle_help, handle_link, handle_new,
    handle_session, handle_sessions, handle_status, handle_stream,
};
use bot::{BotState, Command};
use config::Config;
use dptree::case;

use forum::TopicStore;
use integration::Integration;
use opencode::stream_handler::StreamHandler;
use opencode::OpenCodeClient;
use orchestrator::manager::InstanceManager;
use orchestrator::port_pool::PortPool;
use orchestrator::store::OrchestratorStore;
use std::sync::Arc;
use std::time::Instant;
use teloxide::prelude::*;
use tokio::signal;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("oc_outpost=info")),
        )
        .init();

    info!("oc-outpost v{}", env!("CARGO_PKG_VERSION"));
    info!("Starting Telegram bot...");

    info!("Initializing databases...");
    let orchestrator_store = OrchestratorStore::new(&config.orchestrator_db_path).await?;
    let topic_store = TopicStore::new(&config.topic_db_path).await?;

    let api_state = api::AppState {
        store: orchestrator_store.clone(),
        api_key: config.api_key.clone(),
    };

    let store_for_manager = orchestrator_store.clone();
    let port_pool = PortPool::new(config.opencode_port_start, config.opencode_port_pool_size);
    let instance_manager =
        InstanceManager::new(Arc::new(config.clone()), store_for_manager, port_pool).await?;
    let bot_start_time = Instant::now();

    let bot_state = Arc::new(BotState::new(
        orchestrator_store,
        topic_store,
        config.clone(),
        instance_manager,
        bot_start_time,
    ));
    let api_router = api::create_router(api_state);
    let api_addr = format!("127.0.0.1:{}", config.api_port);
    let api_listener = tokio::net::TcpListener::bind(&api_addr).await?;
    info!("API server listening on http://{}", api_addr);

    let api_handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(api_listener, api_router).await {
            error!("API server error: {}", e);
        }
    });

    let bot = Bot::new(&config.telegram_bot_token);

    let opencode_client =
        OpenCodeClient::new(&format!("http://localhost:{}", config.opencode_port_start));
    let stream_handler = Arc::new(StreamHandler::new(opencode_client));

    let integration = Arc::new(Integration::new(bot_state.clone(), stream_handler));

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<Command>()
                .branch(case![Command::New(name)].endpoint({
                    let state = Arc::clone(&bot_state);
                    move |bot: Bot, msg: Message, cmd: Command| {
                        let state = Arc::clone(&state);
                        async move {
                            if let Err(e) = handle_new(bot, msg, cmd, state).await {
                                error!("Error handling /new: {:?}", e);
                            }
                            respond(())
                        }
                    }
                }))
                .branch(case![Command::Sessions].endpoint({
                    let state = Arc::clone(&bot_state);
                    move |bot: Bot, msg: Message, cmd: Command| {
                        let state = Arc::clone(&state);
                        async move {
                            if let Err(e) = handle_sessions(bot, msg, cmd, state).await {
                                error!("Error handling /sessions: {:?}", e);
                            }
                            respond(())
                        }
                    }
                }))
                .branch(case![Command::Connect(id)].endpoint({
                    let state = Arc::clone(&bot_state);
                    move |bot: Bot, msg: Message, cmd: Command| {
                        let state = Arc::clone(&state);
                        async move {
                            if let Err(e) = handle_connect(bot, msg, cmd, state).await {
                                error!("Error handling /connect: {:?}", e);
                            }
                            respond(())
                        }
                    }
                }))
                .branch(case![Command::Disconnect].endpoint({
                    let state = Arc::clone(&bot_state);
                    move |bot: Bot, msg: Message, cmd: Command| {
                        let state = Arc::clone(&state);
                        async move {
                            if let Err(e) = handle_disconnect(bot, msg, cmd, state).await {
                                error!("Error handling /disconnect: {:?}", e);
                            }
                            respond(())
                        }
                    }
                }))
                .branch(case![Command::Link(path)].endpoint({
                    let state = Arc::clone(&bot_state);
                    move |bot: Bot, msg: Message, cmd: Command| {
                        let state = Arc::clone(&state);
                        async move {
                            if let Err(e) = handle_link(bot, msg, cmd, state).await {
                                error!("Error handling /link: {:?}", e);
                            }
                            respond(())
                        }
                    }
                }))
                .branch(case![Command::Stream].endpoint({
                    let state = Arc::clone(&bot_state);
                    move |bot: Bot, msg: Message, cmd: Command| {
                        let state = Arc::clone(&state);
                        async move {
                            if let Err(e) = handle_stream(bot, msg, cmd, state).await {
                                error!("Error handling /stream: {:?}", e);
                            }
                            respond(())
                        }
                    }
                }))
                .branch(case![Command::Session].endpoint({
                    let state = Arc::clone(&bot_state);
                    move |bot: Bot, msg: Message, cmd: Command| {
                        let state = Arc::clone(&state);
                        async move {
                            if let Err(e) = handle_session(bot, msg, cmd, state).await {
                                error!("Error handling /session: {:?}", e);
                            }
                            respond(())
                        }
                    }
                }))
                .branch(case![Command::Status].endpoint({
                    let state = Arc::clone(&bot_state);
                    move |bot: Bot, msg: Message, cmd: Command| {
                        let state = Arc::clone(&state);
                        async move {
                            if let Err(e) = handle_status(bot, msg, cmd, state).await {
                                error!("Error handling /status: {:?}", e);
                            }
                            respond(())
                        }
                    }
                }))
                .branch(case![Command::Clear].endpoint({
                    let state = Arc::clone(&bot_state);
                    move |bot: Bot, msg: Message, cmd: Command| {
                        let state = Arc::clone(&state);
                        async move {
                            if let Err(e) = handle_clear(bot, msg, cmd, state).await {
                                error!("Error handling /clear: {:?}", e);
                            }
                            respond(())
                        }
                    }
                }))
                .branch(case![Command::Help].endpoint({
                    let state = Arc::clone(&bot_state);
                    move |bot: Bot, msg: Message, cmd: Command| {
                        let state = Arc::clone(&state);
                        async move {
                            if let Err(e) = handle_help(bot, msg, cmd, state).await {
                                error!("Error handling /help: {:?}", e);
                            }
                            respond(())
                        }
                    }
                })),
        )
        .branch(Update::filter_message().endpoint({
            let integration = Arc::clone(&integration);
            move |bot: Bot, msg: Message| {
                let integration = Arc::clone(&integration);
                async move {
                    if let Err(e) = integration.handle_message(bot, msg).await {
                        error!("Error handling message: {:?}", e);
                    }
                    respond(())
                }
            }
        }));

    let mut dispatcher = Dispatcher::builder(bot.clone(), handler)
        .dependencies(dptree::deps![bot_state])
        .enable_ctrlc_handler()
        .build();

    info!("Bot connected. Press Ctrl+C to stop.");

    tokio::select! {
        _ = dispatcher.dispatch() => {
            info!("Dispatcher stopped");
        }
        _ = signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down gracefully...");
        }
    }

    info!("Stopping active streams...");
    integration.stop_all_streams().await;

    info!("Stopping API server...");
    api_handle.abort();

    info!("Shutdown complete.");
    Ok(())
}
