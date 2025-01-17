use crate::data::Data;
use antiraid_types::punishments::Punishment;
use antiraid_types::stings::Sting;
use antiraid_types::userinfo::UserInfo;
use strum::{IntoStaticStr, VariantNames};

pub use typetag; // Re-exported

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BuiltinCommandExecuteData {
    pub command: String,
    pub user_id: serenity::all::UserId,
    pub user_info: UserInfo,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PermissionCheckData {
    pub perm: kittycat::perms::Permission,
    pub user_id: serenity::all::UserId,
    pub user_info: UserInfo,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "action")]
pub enum ModerationAction {
    Kick {
        member: serenity::all::Member, // The target to kick
    },
    TempBan {
        user: serenity::all::User, // The target to ban
        duration: u64,             // Duration, in seconds
        prune_dmd: u8,
    },
    Ban {
        user: serenity::all::User, // The target to ban
        prune_dmd: u8,
    },
    Unban {
        user: serenity::all::User, // The target to unban
    },
    Timeout {
        member: serenity::all::Member, // The target to timeout
        duration: u64,                 // Duration, in seconds
    },
    Prune {
        user: Option<serenity::all::User>,
        prune_opts: serde_json::Value,
        channels: Vec<serenity::all::ChannelId>,
    },
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ModerationStartEventData {
    pub correlation_id: sqlx::types::Uuid, // This will also be sent on ModerationEndEventData to correlate the events while avoiding duplication of data
    pub action: ModerationAction,
    pub author: serenity::all::Member,
    pub num_stings: i32,
    pub reason: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ModerationEndEventData {
    pub correlation_id: sqlx::types::Uuid, // Will correlate with a ModerationStart's event data
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum ExternalKeyUpdateEventDataAction {
    Create,
    Update,
    Delete,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ExternalKeyUpdateEventData {
    pub key_modified: String,
    pub author: serenity::all::UserId,
    pub action: ExternalKeyUpdateEventDataAction,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, IntoStaticStr, VariantNames)]
#[must_use]
pub enum AntiraidEvent {
    /// A sting create event. Dispatched when a sting is created
    StingCreate(Sting),

    /// A sting update event. Dispatched when a sting is updated
    StingUpdate(Sting),

    /// A sting expiry event. Dispatched when a sting expires
    StingExpire(Sting),

    /// A sting delete event. Dispatched when a sting is manually deleted
    StingDelete(Sting),

    /// A punishment create event. Dispatched when a punishment is created
    PunishmentCreate(Punishment),

    /// A punishment expiration event. Dispatched when a punishment expires
    PunishmentExpire(Punishment),

    /// A punishment delete event. Dispatched when a punishment is manually deleted
    PunishmentDelete(Punishment),

    /// An on startup event is fired when a set of templates are modified
    ///
    /// The inner Vec<String> is the list of templates modified/reloaded
    OnStartup(Vec<String>),

    /// A builtin command execute event is fired when a core/builtin command is executed
    ///
    /// This contains three fields, the command name, the user id and the UserInfo
    BuiltinCommandExecute(BuiltinCommandExecuteData),

    /// A permission check event is fired when a permission check is done
    PermissionCheckExecute(PermissionCheckData),

    /// A moderation start event is fired prior to the execution of a moderation action
    ModerationStart(ModerationStartEventData),

    /// A moderation end event is fired after the execution of a moderation action
    ///
    /// Note that this event is not guaranteed to be fired (e.g. the action fails, jobserver timeout etc.)
    ModerationEnd(ModerationEndEventData),

    /// A key external modify event. Fired when a key is modified externally
    ExternalKeyUpdate(ExternalKeyUpdateEventData),
}

impl std::fmt::Display for AntiraidEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s: &'static str = self.into();
        write!(f, "{}", s)
    }
}

impl AntiraidEvent {
    /// Returns the variant names
    pub fn variant_names() -> &'static [&'static str] {
        Self::VARIANTS
    }

    /// Convert the event's inner data to a JSON value
    pub fn to_value(&self) -> Result<serde_json::Value, serde_json::Error> {
        match self {
            AntiraidEvent::StingCreate(sting) => serde_json::to_value(sting),
            AntiraidEvent::StingUpdate(sting) => serde_json::to_value(sting),
            AntiraidEvent::StingExpire(sting) => serde_json::to_value(sting),
            AntiraidEvent::StingDelete(sting) => serde_json::to_value(sting),
            AntiraidEvent::PunishmentCreate(punishment) => serde_json::to_value(punishment),
            AntiraidEvent::PunishmentExpire(punishment) => serde_json::to_value(punishment),
            AntiraidEvent::PunishmentDelete(punishment) => serde_json::to_value(punishment),
            AntiraidEvent::OnStartup(templates) => serde_json::to_value(templates),
            AntiraidEvent::BuiltinCommandExecute(data) => serde_json::to_value(data),
            AntiraidEvent::PermissionCheckExecute(data) => serde_json::to_value(data),
            AntiraidEvent::ModerationStart(data) => serde_json::to_value(data),
            AntiraidEvent::ModerationEnd(data) => serde_json::to_value(data),
            AntiraidEvent::ExternalKeyUpdate(data) => serde_json::to_value(data),
        }
    }

    /// Returns the author of the event
    pub fn author(&self) -> Option<String> {
        match self {
            AntiraidEvent::StingCreate(sting) => Some(sting.creator.to_string()),
            AntiraidEvent::StingUpdate(sting) => Some(sting.creator.to_string()),
            AntiraidEvent::StingExpire(sting) => Some(sting.creator.to_string()),
            AntiraidEvent::StingDelete(sting) => Some(sting.creator.to_string()), // For now
            AntiraidEvent::PunishmentCreate(punishment) => Some(punishment.creator.to_string()),
            AntiraidEvent::PunishmentExpire(punishment) => Some(punishment.creator.to_string()),
            AntiraidEvent::PunishmentDelete(punishment) => Some(punishment.creator.to_string()), // For now
            AntiraidEvent::OnStartup(_) => None,
            AntiraidEvent::BuiltinCommandExecute(be) => Some(be.user_id.to_string()),
            AntiraidEvent::PermissionCheckExecute(pce) => Some(pce.user_id.to_string()),
            AntiraidEvent::ModerationStart(data) => Some(data.author.user.id.to_string()),
            AntiraidEvent::ModerationEnd(_) => None,
            AntiraidEvent::ExternalKeyUpdate(data) => Some(data.author.to_string()),
        }
    }
}

impl AntiraidEvent {
    /// Dispatch the event to the template worker process
    pub async fn dispatch_to_template_worker_and_nowait(
        &self,
        data: &Data,
        guild_id: serenity::all::GuildId,
    ) -> Result<(), crate::Error> {
        let url = format!(
            "http://{}:{}/dispatch-event/{}",
            config::CONFIG.base_ports.template_worker_addr,
            config::CONFIG.base_ports.template_worker_port,
            guild_id
        );

        let resp = data.reqwest.post(&url).json(&self).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            let err_text = resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            Err(err_text.into())
        }
    }

    /// Dispatch the event to the template worker process
    pub async fn dispatch_to_template_worker_and_wait(
        &self,
        data: &Data,
        guild_id: serenity::all::GuildId,
        wait_timeout: std::time::Duration,
    ) -> Result<AntiraidEventResultHandle, crate::Error> {
        let url = format!(
            "http://{}:{}/dispatch-event/{}/@wait?wait_timeout={}",
            config::CONFIG.base_ports.template_worker_addr,
            config::CONFIG.base_ports.template_worker_port,
            guild_id,
            wait_timeout.as_millis()
        );

        let resp = data.reqwest.post(&url).json(&self).send().await?;

        if resp.status().is_success() {
            let json = resp.json::<Vec<serde_json::Value>>().await?;

            Ok(AntiraidEventResultHandle { results: json })
        } else {
            let err_text = resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            Err(err_text.into())
        }
    }
}

pub struct AntiraidEventResultHandle {
    pub results: Vec<serde_json::Value>,
}

impl std::ops::Deref for AntiraidEventResultHandle {
    type Target = Vec<serde_json::Value>;

    fn deref(&self) -> &Self::Target {
        &self.results
    }
}

impl AntiraidEventResultHandle {
    /// Returns if at least one template has a "allow_exec" set to true
    ///
    /// This means the template explicitly allows for execution to occur without falling back to default checks
    ///
    /// Note that this does not mean to deny the op (an error should be used for that)
    pub fn can_execute(&self) -> bool {
        for result in &self.results {
            if let Some(result) = result.get("allow_exec") {
                if result.as_bool().unwrap_or_default() {
                    return true;
                }
            }
        }

        false
    }
}
