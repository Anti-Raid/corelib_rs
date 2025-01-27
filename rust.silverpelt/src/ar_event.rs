use std::collections::HashMap;

use crate::data::Data;
use antiraid_types::ar_event::AntiraidEvent;

#[allow(async_fn_in_trait)]
pub trait AntiraidEventOperations {
    /// Dispatch the event to the template worker process
    async fn dispatch_to_template_worker_and_nowait(
        &self,
        data: &Data,
        guild_id: serenity::all::GuildId,
    ) -> Result<(), crate::Error>;

    /// Dispatch the event to the template worker process
    async fn dispatch_to_template_worker_and_wait(
        &self,
        data: &Data,
        guild_id: serenity::all::GuildId,
        wait_timeout: std::time::Duration,
    ) -> Result<AntiraidEventResultHandle, crate::Error>;
}

impl AntiraidEventOperations for AntiraidEvent {
    /// Dispatch the event to the template worker process
    async fn dispatch_to_template_worker_and_nowait(
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
    async fn dispatch_to_template_worker_and_wait(
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
            let json = resp.json::<HashMap<String, serde_json::Value>>().await?;

            // Check for DispatchStop
            for result in json.values() {
                if let Some(value) = result.get("DispatchStop") {
                    match value {
                        serde_json::Value::String(s) => return Err(s.clone().into()),
                        value => return Err(value.to_string().into()),
                    }
                }
            }

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
    pub results: HashMap<String, serde_json::Value>,
}

impl std::ops::Deref for AntiraidEventResultHandle {
    type Target = HashMap<String, serde_json::Value>;

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
        for result in self.results.values() {
            if let Some(result) = result.get("allow_exec") {
                if result.as_bool().unwrap_or_default() {
                    return true;
                }
            }
        }

        false
    }
}
