use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "payload")]
pub enum DomainEvent {
    AssetCreated { id: String, path: Option<String> },
    AssetUpdated { id: String },
    AssetDeleted { id: String },
    EntityCreated { id: String, title: String },
    EntityUpdated { id: String },
    EntityDeleted { id: String },
    MemoryCreated { id: String, title: String },
    MemoryUpdated { id: String },
    MemoryDeleted { id: String },
    RelationCreated { id: String, source_id: String, target_id: String },
    RelationDeleted { id: String },
    ExternalReferenceCreated { id: String, url: String },
    ExternalReferenceDeleted { id: String },
    BackupCompleted { path: String, duration_ms: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub event: DomainEvent,
}

impl EventEnvelope {
    pub fn new(actor: impl Into<String>, event: DomainEvent) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            actor: actor.into(),
            event,
        }
    }
}

pub type EventHandler = Box<dyn Fn(&EventEnvelope) + Send + Sync + 'static>;

#[derive(Default, Clone)]
pub struct EventBus {
    handlers: Arc<Mutex<Vec<EventHandler>>>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn subscribe<F>(&self, handler: F)
    where
        F: Fn(&EventEnvelope) + Send + Sync + 'static,
    {
        if let Ok(mut lock) = self.handlers.lock() {
            lock.push(Box::new(handler));
        }
    }

    pub fn publish(&self, envelope: EventEnvelope) {
        if let Ok(lock) = self.handlers.lock() {
            for handler in lock.iter() {
                handler(&envelope);
            }
        }
    }
}
