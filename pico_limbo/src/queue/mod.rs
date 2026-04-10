pub mod placeholder;
pub mod queue_display;

use crate::configuration::queue::QueueConfig;
use minecraft_protocol::prelude::Uuid;
use std::collections::VecDeque;
use tokio::sync::Mutex;
use tokio::sync::oneshot;

pub enum PushAction {
    Kick(String),
    Transfer { host: String, port: i32 },
}

struct QueuePlayer {
    uuid: Uuid,
    username: String,
    push_tx: Option<oneshot::Sender<PushAction>>,
}

pub struct QueueSettings {
    pub push_interval_seconds: u64,
    pub push_count: usize,
    pub refresh_interval_seconds: u64,
    pub push_action: PushActionTemplate,
}

pub enum PushActionTemplate {
    Kick { message: String },
    Transfer { host: String, port: i32 },
}

pub struct QueueState {
    players: Mutex<VecDeque<QueuePlayer>>,
    pub settings: QueueSettings,
}

impl QueueState {
    pub fn from_config(config: &QueueConfig) -> Self {
        use crate::configuration::queue::QueuePushMethodConfig;
        let push_action = match &config.push_method {
            QueuePushMethodConfig::Kick { kick_message } => PushActionTemplate::Kick {
                message: kick_message.clone(),
            },
            QueuePushMethodConfig::Transfer { host, port } => PushActionTemplate::Transfer {
                host: host.clone(),
                port: i32::from(*port),
            },
        };

        Self {
            players: Mutex::new(VecDeque::new()),
            settings: QueueSettings {
                push_interval_seconds: config.push_interval_seconds,
                push_count: config.push_count,
                refresh_interval_seconds: config.refresh_interval_seconds,
                push_action,
            },
        }
    }

    /// Adds a player to the back of the queue. Returns their 1-based position.
    pub async fn enqueue(
        &self,
        uuid: Uuid,
        username: String,
        push_tx: oneshot::Sender<PushAction>,
    ) -> usize {
        let mut players = self.players.lock().await;
        players.push_back(QueuePlayer {
            uuid,
            username,
            push_tx: Some(push_tx),
        });
        players.len()
    }

    /// Removes a player from the queue (on disconnect or push).
    pub async fn dequeue(&self, uuid: Uuid) {
        let mut players = self.players.lock().await;
        players.retain(|p| p.uuid != uuid);
    }

    /// Returns the 1-based position of the player, or None if not in queue.
    pub async fn position_of(&self, uuid: Uuid) -> Option<usize> {
        let players = self.players.lock().await;
        players
            .iter()
            .position(|p| p.uuid == uuid)
            .map(|i| i + 1)
    }

    /// Returns the total number of players in the queue.
    pub async fn total(&self) -> usize {
        self.players.lock().await.len()
    }

    /// Pops up to `count` players from the front and sends them the push action.
    pub async fn push_next(&self, count: usize) {
        let mut players = self.players.lock().await;
        for _ in 0..count {
            if let Some(mut player) = players.pop_front() {
                let action = match &self.settings.push_action {
                    PushActionTemplate::Kick { message } => PushAction::Kick(message.clone()),
                    PushActionTemplate::Transfer { host, port } => PushAction::Transfer {
                        host: host.clone(),
                        port: *port,
                    },
                };
                if let Some(tx) = player.push_tx.take() {
                    let _ = tx.send(action);
                }
            } else {
                break;
            }
        }
    }

    /// Returns the username of a player at the given position (for display).
    pub async fn username_of(&self, uuid: Uuid) -> Option<String> {
        let players = self.players.lock().await;
        players
            .iter()
            .find(|p| p.uuid == uuid)
            .map(|p| p.username.clone())
    }
}
