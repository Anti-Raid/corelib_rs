use crate::data::Data;

pub use typetag; // Re-exported

/// This can be used to trigger a custom event
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CustomEvent {
    pub event_name: String,
    pub event_titlename: String,
    pub event_data: serde_json::Value,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BuiltinCommandExecuteData {
    pub command: String,
    pub user_id: serenity::all::UserId,
    pub user_info: crate::userinfo::UserInfo,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PermissionCheckData {
    pub perm: kittycat::perms::Permission,
    pub user_id: serenity::all::UserId,
    pub user_info: crate::userinfo::UserInfo,
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
#[must_use]
pub enum AntiraidEvent {
    /// A sting create event. Dispatched when a sting is created
    StingCreate(super::stings::Sting),

    /// A sting update event. Dispatched when a sting is updated
    StingUpdate(super::stings::Sting),

    /// A sting expiry event. Dispatched when a sting expires
    StingExpire(super::stings::Sting),

    /// A sting delete event. Dispatched when a sting is manually deleted
    StingDelete(super::stings::Sting),

    /// A punishment create event. Dispatched when a punishment is created
    PunishmentCreate(super::punishments::Punishment),

    /// A punishment expiration event. Dispatched when a punishment expires
    PunishmentExpire(super::punishments::Punishment),

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
