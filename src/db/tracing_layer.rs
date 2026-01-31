use std::collections::HashMap;
use std::fmt::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use super::log_store::LogStore;
use tokio::runtime::Handle;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

pub struct DatabaseLayer {
    store: LogStore,
    handle: Handle,
    run_id: String,
}

impl DatabaseLayer {
    pub fn new(store: LogStore, handle: Handle, run_id: String) -> Self {
        Self {
            store,
            handle,
            run_id,
        }
    }
}

impl<S> Layer<S> for DatabaseLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let level = metadata.level().as_str();
        let target = metadata.target();

        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);

        let message = visitor.message;
        let fields = if visitor.fields.is_empty() {
            None
        } else {
            serde_json::to_string(&visitor.fields).ok()
        };

        let store = self.store.clone();
        let run_id = self.run_id.clone();
        let level = level.to_owned();
        let target = target.to_owned();

        self.handle.spawn(async move {
            let _ = store
                .insert_log(
                    &run_id,
                    timestamp,
                    &level,
                    &target,
                    &message,
                    fields.as_deref(),
                )
                .await;
        });
    }
}

#[derive(Default)]
struct EventVisitor {
    message: String,
    fields: HashMap<String, String>,
}

impl Visit for EventVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        match field.name() {
            "message" => {
                let _ = write!(self.message, "{value:?}");
            }
            name => {
                self.fields.insert(name.to_owned(), format!("{value:?}"));
            }
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "message" => {
                self.message = value.to_owned();
            }
            name => {
                self.fields.insert(name.to_owned(), value.to_owned());
            }
        }
    }
}
