use lockdowns::{
    from_lockdown_mode_string, CreateLockdown, GuildLockdownSettings, Lockdown, LockdownDataStore,
};
use sandwich_driver::SandwichConfigData;

pub struct LockdownData<'a> {
    pub cache: &'a serenity::all::Cache,
    pub http: &'a serenity::all::Http,
    pub pool: sqlx::PgPool,
    pub reqwest: reqwest::Client,
    pub sandwich_config: SandwichConfigData,
}

impl<'a> LockdownData<'a> {
    pub fn new(
        cache: &'a serenity::all::Cache,
        http: &'a serenity::all::Http,
        pool: sqlx::PgPool,
        reqwest: reqwest::Client,
        sandwich_config: SandwichConfigData,
    ) -> Self {
        Self {
            cache,
            http,
            pool,
            reqwest,
            sandwich_config,
        }
    }
}

impl LockdownDataStore for LockdownData<'_> {
    async fn get_guild_lockdown_settings(
        &self,
        guild_id: serenity::all::GuildId,
    ) -> Result<lockdowns::GuildLockdownSettings, lockdowns::Error> {
        match sqlx::query!(
            "SELECT member_roles, require_correct_layout FROM lockdown__guilds WHERE guild_id = $1",
            guild_id.to_string(),
        )
        .fetch_optional(&self.pool)
        .await?
        {
            Some(settings) => {
                let member_roles = settings
                    .member_roles
                    .iter()
                    .map(|r| r.parse().unwrap())
                    .collect();

                let settings = GuildLockdownSettings {
                    member_roles,
                    require_correct_layout: settings.require_correct_layout,
                };

                Ok(settings)
            }
            None => Ok(GuildLockdownSettings::default()),
        }
    }

    async fn guild(
        &self,
        guild_id: serenity::all::GuildId,
    ) -> Result<serenity::all::PartialGuild, lockdowns::Error> {
        sandwich_driver::guild(
            self.cache,
            self.http,
            &self.reqwest,
            guild_id,
            &self.sandwich_config,
        )
        .await
    }

    async fn guild_channels(
        &self,
        guild_id: serenity::all::GuildId,
    ) -> Result<Vec<serenity::all::GuildChannel>, lockdowns::Error> {
        sandwich_driver::guild_channels(
            self.cache,
            self.http,
            &self.reqwest,
            guild_id,
            &self.sandwich_config,
        )
        .await
    }

    fn cache(&self) -> Option<&serenity::all::Cache> {
        Some(self.cache)
    }

    fn http(&self) -> &serenity::all::Http {
        self.http
    }

    async fn get_lockdowns(
        &self,
        guild_id: serenity::all::GuildId,
    ) -> Result<Vec<Lockdown>, lockdowns::Error> {
        let data = sqlx::query!(
            "SELECT id, type, data, reason, created_at FROM lockdown__guild_lockdowns WHERE guild_id = $1",
            guild_id.to_string(),
        )
        .fetch_all(&self.pool)
        .await?;

        let mut lockdowns = Vec::new();

        for row in data {
            let id = row.id;
            let r#type = row.r#type;
            let data = row.data;
            let reason = row.reason;
            let created_at = row.created_at;

            let lockdown_mode = from_lockdown_mode_string(&r#type)?;

            let lockdown = Lockdown {
                id,
                r#type: lockdown_mode,
                data,
                reason,
                created_at,
            };

            lockdowns.push(lockdown);
        }

        Ok(lockdowns)
    }

    async fn insert_lockdown(
        &self,
        guild_id: serenity::all::GuildId,
        lockdown: CreateLockdown,
    ) -> Result<Lockdown, lockdowns::Error> {
        let id = sqlx::query!(
            "INSERT INTO lockdown__guild_lockdowns (guild_id, type, data, reason) VALUES ($1, $2, $3, $4) RETURNING id, created_at",
            guild_id.to_string(),
            lockdown.r#type.string_form(),
            &lockdown.data,
            lockdown.reason.clone(),
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Lockdown {
            id: id.id,
            r#type: lockdown.r#type,
            data: lockdown.data,
            reason: lockdown.reason,
            created_at: id.created_at,
        })
    }

    async fn remove_lockdown(
        &self,
        guild_id: serenity::all::GuildId,
        id: uuid::Uuid,
    ) -> Result<(), lockdowns::Error> {
        sqlx::query!(
            "DELETE FROM lockdown__guild_lockdowns WHERE guild_id = $1 AND id = $2",
            guild_id.to_string(),
            id,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
