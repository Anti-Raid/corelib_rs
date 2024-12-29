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

    /// An on startup event is fired *at least once* when the bot starts up or the set of templates are modified
    ///
    /// The inner Vec<String> is the list of templates modified/reloaded
    OnStartup(Vec<String>),

    /// A custom event
    Custom(CustomEvent),
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
    ) -> Result<Vec<serde_json::Value>, crate::Error> {
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

            Ok(json)
        } else {
            let err_text = resp
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            Err(err_text.into())
        }
    }
}
