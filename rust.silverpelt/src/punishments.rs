use antiraid_types::punishments::{
    Punishment, PunishmentCreate, PunishmentState, PunishmentTarget,
};
use std::str::FromStr;

use crate::{
    ar_event::{AntiraidEventOperations, DispatchEventData},
    pginterval::pg_interval_to_secs,
};
use sqlx::{postgres::types::PgInterval, Row};

#[allow(async_fn_in_trait)]
pub trait PunishmentOperations: Send + Sync {
    /// Returns a punishment by ID
    async fn get(
        db: impl sqlx::PgExecutor<'_>,
        guild_id: serenity::all::GuildId,
        id: sqlx::types::Uuid,
    ) -> Result<Option<Punishment>, crate::Error>;

    /// Lists punishments for a guild paginated based on page number
    async fn list(
        db: impl sqlx::PgExecutor<'_>,
        guild_id: serenity::all::GuildId,
        page: usize,
    ) -> Result<Vec<Punishment>, crate::Error>;

    /// Get all expired punishments
    async fn get_expired(db: impl sqlx::PgExecutor<'_>) -> Result<Vec<Punishment>, crate::Error>;

    /// Dispatch a PunishmentCreate event
    async fn dispatch_event(
        self,
        ctx: serenity::all::Context,
        dispatch_event_data: &DispatchEventData,
    ) -> Result<(), crate::Error>;
}

#[derive(sqlx::FromRow)]
struct PunishmentRow {
    id: uuid::Uuid,
    src: Option<String>,
    guild_id: String,
    punishment: String,
    creator: String,
    target: String,
    state: String,
    handle_log: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
    duration: Option<PgInterval>,
    reason: String,
    data: Option<serde_json::Value>,
}

impl PunishmentRow {
    fn into_punishment(self) -> Result<Punishment, crate::Error> {
        Ok(Punishment {
            id: self.id,
            src: self.src,
            guild_id: self.guild_id.parse()?,
            punishment: self.punishment,
            creator: PunishmentTarget::from_str(&self.creator)?,
            target: PunishmentTarget::from_str(&self.target)?,
            state: PunishmentState::from_str(&self.state)?,
            handle_log: self.handle_log,
            created_at: self.created_at,
            duration: match self.duration {
                Some(d) => {
                    let secs = pg_interval_to_secs(d);
                    Some(std::time::Duration::from_secs(secs.try_into()?))
                }
                None => None,
            },
            reason: self.reason,
            data: self.data,
        })
    }
}

impl PunishmentOperations for Punishment {
    /// Returns a punishment by ID
    async fn get(
        db: impl sqlx::PgExecutor<'_>,
        guild_id: serenity::all::GuildId,
        id: sqlx::types::Uuid,
    ) -> Result<Option<Punishment>, crate::Error> {
        let rec = sqlx::query_as(
            "SELECT id, src, guild_id, punishment, creator, target, state, handle_log, created_at, duration, reason, data FROM punishments WHERE id = $1 AND guild_id = $2",
        )
        .bind(id)
        .bind(guild_id.to_string())
        .fetch_optional(db)
        .await?;

        match rec {
            Some(row) => {
                let row: PunishmentRow = row;
                let punishment = row.into_punishment()?;
                Ok(Some(punishment))
            }
            None => Ok(None),
        }
    }

    /// Lists punishments for a guild paginated based on page number
    async fn list(
        db: impl sqlx::PgExecutor<'_>,
        guild_id: serenity::all::GuildId,
        page: usize,
    ) -> Result<Vec<Punishment>, crate::Error> {
        const PAGE_SIZE: i64 = 20; // 20 punishments per page

        let rec: Vec<PunishmentRow> = sqlx::query_as(
            "SELECT id, src, guild_id, punishment, creator, target, state, handle_log, created_at, duration, reason, data FROM punishments WHERE guild_id = $1 ORDER BY created_at DESC OFFSET $2 LIMIT $3",
        )
        .bind(guild_id.to_string())
        .bind((page as i64 - 1) * PAGE_SIZE)
        .bind(PAGE_SIZE)
        .fetch_all(db)
        .await?;

        let mut punishments = Vec::new();

        for row in rec {
            let punishment = row.into_punishment()?;
            punishments.push(punishment);
        }

        Ok(punishments)
    }

    async fn get_expired(db: impl sqlx::PgExecutor<'_>) -> Result<Vec<Punishment>, crate::Error> {
        let rec: Vec<PunishmentRow> = sqlx::query_as(
            "SELECT id, src, guild_id, punishment, creator, target, state, handle_log, created_at, duration, reason, data FROM punishments WHERE duration IS NOT NULL AND state = 'active' AND (created_at + duration) < NOW()",
        )
        .fetch_all(db)
        .await?;

        let mut punishments = Vec::new();

        for row in rec {
            let punishment = row.into_punishment()?;
            punishments.push(punishment);
        }

        Ok(punishments)
    }

    /// Dispatch a PunishmentCreate event
    async fn dispatch_event(
        self,
        ctx: serenity::all::Context,
        dispatch_event_data: &DispatchEventData,
    ) -> Result<(), crate::Error> {
        let guild_id = self.guild_id;
        antiraid_types::ar_event::AntiraidEvent::PunishmentCreate(self)
            .dispatch_to_template_worker_and_nowait(
                &ctx.data::<crate::data::Data>(),
                guild_id,
                dispatch_event_data,
            )
            .await?;

        Ok(())
    }
}

#[allow(async_fn_in_trait)]
pub trait PunishmentCreateOperations: Send + Sync {
    /// Creates a new Punishment without dispatching it as an event
    async fn create_without_dispatch(
        self,
        db: impl sqlx::PgExecutor<'_>,
    ) -> Result<Punishment, crate::Error>;

    /// Creates a new Punishment and dispatches it as an event in one go
    async fn create_and_dispatch(
        self,
        ctx: serenity::all::Context,
        db: impl sqlx::PgExecutor<'_>,
        dispatch_event_data: &DispatchEventData,
    ) -> Result<(), crate::Error>;

    /// Creates a new Punishment and dispatches it as an event in one go
    async fn create_and_dispatch_returning_id(
        self,
        ctx: serenity::all::Context,
        db: impl sqlx::PgExecutor<'_>,
        dispatch_event_data: &DispatchEventData,
    ) -> Result<sqlx::types::Uuid, crate::Error>;
}

impl PunishmentCreateOperations for PunishmentCreate {
    /// Creates a new Punishment without dispatching it as an event
    async fn create_without_dispatch(
        self,
        db: impl sqlx::PgExecutor<'_>,
    ) -> Result<Punishment, crate::Error> {
        let ret_data = sqlx::query(
            r#"
            INSERT INTO punishments (src, guild_id, punishment, creator, target, handle_log, duration, reason, data, state)
            VALUES ($1, $2, $3, $4, $5, $6, make_interval(secs => $7), $8, $9, $10) RETURNING id, created_at
            "#,
        )
        .bind(&self.src)
        .bind(self.guild_id.to_string())
        .bind(&self.punishment)
        .bind(self.creator.to_string())
        .bind(self.target.to_string())
        .bind(&self.handle_log)
        .bind(self.duration.map(|d| d.as_secs() as f64))
        .bind(&self.reason)
        .bind(&self.data)
        .bind(self.state.to_string())
        .fetch_one(db)
        .await?;

        Ok(self.to_punishment(ret_data.try_get("id")?, ret_data.try_get("created_at")?))
    }

    /// Creates a new Punishment and dispatches it as an event in one go
    async fn create_and_dispatch(
        self,
        ctx: serenity::all::Context,
        db: impl sqlx::PgExecutor<'_>,
        dispatch_event_data: &DispatchEventData,
    ) -> Result<(), crate::Error> {
        let punishment = self.create_without_dispatch(db).await?;

        punishment.dispatch_event(ctx, dispatch_event_data).await?;

        Ok(())
    }

    /// Creates a new Punishment and dispatches it as an event in one go
    async fn create_and_dispatch_returning_id(
        self,
        ctx: serenity::all::Context,
        db: impl sqlx::PgExecutor<'_>,
        dispatch_event_data: &DispatchEventData,
    ) -> Result<sqlx::types::Uuid, crate::Error> {
        let punishment = self.create_without_dispatch(db).await?;
        let sid = punishment.id;

        punishment.dispatch_event(ctx, dispatch_event_data).await?;

        Ok(sid)
    }
}
