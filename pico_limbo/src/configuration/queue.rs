use crate::configuration::boss_bar::{BossBarColorConfig, BossBarDivisionConfig};
use crate::configuration::require_boolean::{require_false, require_true};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct QueueConfig {
    pub enabled: bool,
    pub push_interval_seconds: u64,
    pub push_count: usize,
    pub refresh_interval_seconds: u64,
    pub push_method: QueuePushMethodConfig,
    pub tab_list: QueueTabListConfig,
    pub title: QueueTitleConfig,
    pub action_bar: QueueActionBarConfig,
    pub boss_bar: QueueBossBarConfig,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            push_interval_seconds: 5,
            push_count: 1,
            refresh_interval_seconds: 3,
            push_method: QueuePushMethodConfig::default(),
            tab_list: QueueTabListConfig::default(),
            title: QueueTitleConfig::default(),
            action_bar: QueueActionBarConfig::default(),
            boss_bar: QueueBossBarConfig::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum QueuePushMethodConfig {
    Kick { kick_message: String },
    Transfer { host: String, port: u16 },
}

impl Default for QueuePushMethodConfig {
    fn default() -> Self {
        Self::Kick {
            kick_message: "You have been moved to the main server.".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum QueueTabListConfig {
    Enabled(EnabledQueueTabListConfig),
    Disabled(DisabledQueueTabListConfig),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct EnabledQueueTabListConfig {
    #[serde(deserialize_with = "require_true")]
    pub enabled: bool,
    pub header: String,
    pub footer: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DisabledQueueTabListConfig {
    #[serde(deserialize_with = "require_false")]
    pub enabled: bool,
}

impl Default for QueueTabListConfig {
    fn default() -> Self {
        Self::Enabled(EnabledQueueTabListConfig {
            enabled: true,
            header: "<bold>Queue</bold>".to_string(),
            footer: "Position: {position}/{total}".to_string(),
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum QueueTitleConfig {
    Enabled(EnabledQueueTitleConfig),
    Disabled(DisabledQueueTitleConfig),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct EnabledQueueTitleConfig {
    #[serde(deserialize_with = "require_true")]
    pub enabled: bool,
    pub title: String,
    pub subtitle: String,
    pub fade_in: i32,
    pub stay: i32,
    pub fade_out: i32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DisabledQueueTitleConfig {
    #[serde(deserialize_with = "require_false")]
    pub enabled: bool,
}

impl Default for QueueTitleConfig {
    fn default() -> Self {
        Self::Enabled(EnabledQueueTitleConfig {
            enabled: true,
            title: "<bold>Queue</bold>".to_string(),
            subtitle: "Position: {position} of {total}".to_string(),
            fade_in: 0,
            stay: 2147483647,
            fade_out: 0,
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum QueueActionBarConfig {
    Enabled(EnabledQueueActionBarConfig),
    Disabled(DisabledQueueActionBarConfig),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct EnabledQueueActionBarConfig {
    #[serde(deserialize_with = "require_true")]
    pub enabled: bool,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DisabledQueueActionBarConfig {
    #[serde(deserialize_with = "require_false")]
    pub enabled: bool,
}

impl Default for QueueActionBarConfig {
    fn default() -> Self {
        Self::Enabled(EnabledQueueActionBarConfig {
            enabled: true,
            text: "<yellow>Queue: {position}/{total} | ETA: {eta}s</yellow>".to_string(),
        })
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum QueueBossBarConfig {
    Enabled(EnabledQueueBossBarConfig),
    Disabled(DisabledQueueBossBarConfig),
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct EnabledQueueBossBarConfig {
    #[serde(deserialize_with = "require_true")]
    pub enabled: bool,
    pub title: String,
    pub color: BossBarColorConfig,
    pub division: BossBarDivisionConfig,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DisabledQueueBossBarConfig {
    #[serde(deserialize_with = "require_false")]
    pub enabled: bool,
}

impl Default for QueueBossBarConfig {
    fn default() -> Self {
        Self::Enabled(EnabledQueueBossBarConfig {
            enabled: true,
            title: "Queue: {position}/{total}".to_string(),
            color: BossBarColorConfig::Blue,
            division: BossBarDivisionConfig::NoDivision,
        })
    }
}
