use anyhow::Result;
use dptree::case;
use oc_outpost::bot::{
    dispatch_callback, handle_close, handle_help, handle_new, handle_projects, handle_session,
    handle_sessions, handle_status,
};
use oc_outpost::bot::{BotState, Command};
use oc_outpost::config::Config;
use oc_outpost::db::log_store::LogStore;
use oc_outpost::db::tracing_layer::DatabaseLayer;
use oc_outpost::forum::TopicStore;
use oc_outpost::integration::Integration;
use oc_outpost::opencode::stream_handler::StreamHandler;
use oc_outpost::opencode::OpenCodeClient;
use oc_outpost::orchestrator::container::DockerRuntime;
use oc_outpost::orchestrator::manager::InstanceManager;
use oc_outpost::orchestrator::port_pool::PortPool;
use oc_outpost::orchestrator::store::OrchestratorStore;
use oc_outpost::types::error::OutpostError;
use std::sync::Arc;
use std::time::Instant;
use teloxide::prelude::*;
use tokio::signal;
use tracing::{debug, error, info, warn};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

fn log_command_error(
    command: &str,
    e: &OutpostError,
    chat_id: i64,
    topic_id: Option<i32>,
    sender_id: Option<u64>,
    sender_username: Option<&str>,
) {
    if e.is_user_error() {
        warn!(
            command = command,
            chat_id = chat_id,
            topic_id = ?topic_id,
            sender_id = ?sender_id,
            sender_username = ?sender_username,
            error = %e,
            "User error handling command"
        );
    } else {
        error!(
            command = command,
            chat_id = chat_id,
            topic_id = ?topic_id,
            sender_id = ?sender_id,
            sender_username = ?sender_username,
            error = %e,
            "Error handling command"
        );
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    debug!("Config loaded from environment");

    let run_id = format!("run_{}", uuid::Uuid::new_v4());
    let version = env!("CARGO_PKG_VERSION");

    let log_store = LogStore::new(&config.log_db_path).await?;
    debug!(log_db = %config.log_db_path.display(), "Log store initialized");

    let config_summary = serde_json::json!({
        "max_instances": config.opencode_max_instances,
        "port_start": config.opencode_port_start,
        "port_pool_size": config.opencode_port_pool_size,
    });
    log_store
        .create_run(&run_id, version, Some(&config_summary.to_string()))
        .await?;

    let default_filter = if cfg!(debug_assertions) {
        "oc_outpost=debug"
    } else {
        "oc_outpost=info"
    };
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default_filter));

    let db_layer = DatabaseLayer::new(
        log_store.clone(),
        tokio::runtime::Handle::current(),
        run_id.clone(),
    );

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .with(db_layer)
        .init();
    debug!("Tracing subscriber initialized with database layer");

    info!(run_id = %run_id, "oc-outpost v{}", version);
    info!("Starting Telegram bot...");

    info!("Initializing databases...");
    let orchestrator_store = OrchestratorStore::new(&config.orchestrator_db_path).await?;
    debug!(db_path = %config.orchestrator_db_path.display(), "Orchestrator store initialized");
    let topic_store = TopicStore::new(&config.topic_db_path).await?;
    debug!(db_path = %config.topic_db_path.display(), "Topic store initialized");

    let store_for_manager = orchestrator_store.clone();
    let port_pool = PortPool::new(config.opencode_port_start, config.opencode_port_pool_size);
    debug!(
        start = config.opencode_port_start,
        size = config.opencode_port_pool_size,
        "Port pool created"
    );
    let runtime = Arc::new(DockerRuntime::new()?);
    let instance_manager = InstanceManager::new(
        Arc::new(config.clone()),
        store_for_manager,
        port_pool,
        runtime,
    )
    .await?;
    debug!("Instance manager created");

    info!("Recovering instances from database...");
    instance_manager.recover_from_db().await?;

    info!("Reconciling containers...");
    if let Err(e) = instance_manager.reconcile_containers().await {
        warn!(error = %e, "Container reconciliation failed (Docker may not be available)");
    }

    info!("Starting health check loop...");
    let _health_check_handle = instance_manager.start_health_check_loop();

    let bot_start_time = Instant::now();

    let bot_state = Arc::new(BotState::new(
        orchestrator_store,
        topic_store,
        config.clone(),
        instance_manager,
        bot_start_time,
    ));
    debug!("Bot state initialized");

    let bot = Bot::new(&config.telegram_bot_token);

    let opencode_client =
        OpenCodeClient::new(&format!("http://localhost:{}", config.opencode_port_start));
    let stream_handler = Arc::new(StreamHandler::new(opencode_client));

    let integration = Arc::new(Integration::new(bot_state.clone(), stream_handler));

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter({
                    let config = config.clone();
                    move |msg: Message| config.is_whitelisted_chat(msg.chat.id.0)
                })
                .branch(
                    dptree::entry()
                        .filter_command::<Command>()
                        .branch(case![Command::New(name)].endpoint({
                            let state = Arc::clone(&bot_state);
                            move |bot: Bot, msg: Message, cmd: Command| {
                                let state = Arc::clone(&state);
                                async move {
                                    let chat_id = msg.chat.id.0;
                                    let topic_id = msg.thread_id.map(|t| t.0 .0);
                                    let sender_id = msg.from.as_ref().map(|u| u.id.0);
                                    let sender_username =
                                        msg.from.as_ref().and_then(|u| u.username.clone());
                                    if let Err(e) = handle_new(bot, msg, cmd, state).await {
                                        log_command_error(
                                            "/new",
                                            &e,
                                            chat_id,
                                            topic_id,
                                            sender_id,
                                            sender_username.as_deref(),
                                        );
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
                                    let chat_id = msg.chat.id.0;
                                    let topic_id = msg.thread_id.map(|t| t.0 .0);
                                    let sender_id = msg.from.as_ref().map(|u| u.id.0);
                                    let sender_username =
                                        msg.from.as_ref().and_then(|u| u.username.clone());
                                    if let Err(e) = handle_sessions(bot, msg, cmd, state).await {
                                        log_command_error(
                                            "/sessions",
                                            &e,
                                            chat_id,
                                            topic_id,
                                            sender_id,
                                            sender_username.as_deref(),
                                        );
                                    }
                                    respond(())
                                }
                            }
                        }))
                        .branch(case![Command::Projects].endpoint({
                            let state = Arc::clone(&bot_state);
                            move |bot: Bot, msg: Message, cmd: Command| {
                                let state = Arc::clone(&state);
                                async move {
                                    let chat_id = msg.chat.id.0;
                                    let topic_id = msg.thread_id.map(|t| t.0 .0);
                                    let sender_id = msg.from.as_ref().map(|u| u.id.0);
                                    let sender_username =
                                        msg.from.as_ref().and_then(|u| u.username.clone());
                                    if let Err(e) = handle_projects(bot, msg, cmd, state).await {
                                        log_command_error(
                                            "/projects",
                                            &e,
                                            chat_id,
                                            topic_id,
                                            sender_id,
                                            sender_username.as_deref(),
                                        );
                                    }
                                    respond(())
                                }
                            }
                        }))
                        .branch(case![Command::Close].endpoint({
                            let state = Arc::clone(&bot_state);
                            move |bot: Bot, msg: Message, cmd: Command| {
                                let state = Arc::clone(&state);
                                async move {
                                    let chat_id = msg.chat.id.0;
                                    let topic_id = msg.thread_id.map(|t| t.0 .0);
                                    let sender_id = msg.from.as_ref().map(|u| u.id.0);
                                    let sender_username =
                                        msg.from.as_ref().and_then(|u| u.username.clone());
                                    if let Err(e) = handle_close(bot, msg, cmd, state).await {
                                        log_command_error(
                                            "/close",
                                            &e,
                                            chat_id,
                                            topic_id,
                                            sender_id,
                                            sender_username.as_deref(),
                                        );
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
                                    let chat_id = msg.chat.id.0;
                                    let topic_id = msg.thread_id.map(|t| t.0 .0);
                                    let sender_id = msg.from.as_ref().map(|u| u.id.0);
                                    let sender_username =
                                        msg.from.as_ref().and_then(|u| u.username.clone());
                                    if let Err(e) = handle_session(bot, msg, cmd, state).await {
                                        log_command_error(
                                            "/session",
                                            &e,
                                            chat_id,
                                            topic_id,
                                            sender_id,
                                            sender_username.as_deref(),
                                        );
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
                                    let chat_id = msg.chat.id.0;
                                    let topic_id = msg.thread_id.map(|t| t.0 .0);
                                    let sender_id = msg.from.as_ref().map(|u| u.id.0);
                                    let sender_username =
                                        msg.from.as_ref().and_then(|u| u.username.clone());
                                    if let Err(e) = handle_status(bot, msg, cmd, state).await {
                                        log_command_error(
                                            "/status",
                                            &e,
                                            chat_id,
                                            topic_id,
                                            sender_id,
                                            sender_username.as_deref(),
                                        );
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
                                    let chat_id = msg.chat.id.0;
                                    let topic_id = msg.thread_id.map(|t| t.0 .0);
                                    let sender_id = msg.from.as_ref().map(|u| u.id.0);
                                    let sender_username =
                                        msg.from.as_ref().and_then(|u| u.username.clone());
                                    if let Err(e) = handle_help(bot, msg, cmd, state).await {
                                        log_command_error(
                                            "/help",
                                            &e,
                                            chat_id,
                                            topic_id,
                                            sender_id,
                                            sender_username.as_deref(),
                                        );
                                    }
                                    respond(())
                                }
                            }
                        })),
                )
                .branch(dptree::entry().endpoint({
                    let integration = Arc::clone(&integration);
                    move |bot: Bot, msg: Message| {
                        let integration = Arc::clone(&integration);
                        async move {
                            let chat_id = msg.chat.id.0;
                            let topic_id = msg.thread_id.map(|t| t.0 .0);
                            let sender_id = msg.from.as_ref().map(|u| u.id.0);
                            let sender_username =
                                msg.from.as_ref().and_then(|u| u.username.clone());

                            if let Err(e) = integration.handle_message(bot, msg).await {
                                if e.is_user_error() {
                                    warn!(
                                        chat_id = chat_id,
                                        topic_id = ?topic_id,
                                        sender_id = ?sender_id,
                                        sender_username = ?sender_username,
                                        error = %e,
                                        "User error handling message"
                                    );
                                } else {
                                    error!(
                                        chat_id = chat_id,
                                        topic_id = ?topic_id,
                                        sender_id = ?sender_id,
                                        sender_username = ?sender_username,
                                        error = %e,
                                        "Error handling message"
                                    );
                                }
                            }
                            respond(())
                        }
                    }
                })),
        )
        .branch(Update::filter_callback_query().endpoint({
            let state = Arc::clone(&bot_state);
            move |bot: Bot, q: CallbackQuery| {
                let state = Arc::clone(&state);
                async move {
                    let sender_id = q.from.id.0;
                    let sender_username = q.from.username.clone();
                    if let Err(e) = dispatch_callback(bot, q, state).await {
                        error!(
                            sender_id = sender_id,
                            sender_username = ?sender_username,
                            error = %e,
                            "Error handling callback"
                        );
                    }
                    respond(())
                }
            }
        }));

    let mut dispatcher = Dispatcher::builder(bot.clone(), handler)
        .dependencies(dptree::deps![bot_state.clone()])
        .enable_ctrlc_handler()
        .build();

    info!("Bot connected. Press Ctrl+C to stop.");
    debug!("Starting Telegram dispatcher loop");

    tokio::select! {
        _ = dispatcher.dispatch() => {
            info!("Dispatcher stopped");
        }
        _ = signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down gracefully...");
        }
    }

    info!("Stopping all OpenCode instances...");
    if let Err(e) = bot_state.instance_manager.stop_all().await {
        error!("Error stopping instances: {:?}", e);
    }

    info!("Stopping active streams...");
    integration.stop_all_streams().await;

    info!("Finishing run log...");
    if let Err(e) = log_store.finish_run(&run_id).await {
        error!("Failed to finalize run log: {:?}", e);
    }

    info!("Shutdown complete.");
    Ok(())
}
