use antiraid_types::punishments::{
    Punishment, PunishmentCreate, PunishmentState, PunishmentTarget,
};
use std::str::FromStr;

use crate::ar_event::{AntiraidEventOperations, DispatchEventData};

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

impl PunishmentOperations for Punishment {
    /// Returns a punishment by ID
    async fn get(
        db: impl sqlx::PgExecutor<'_>,
        guild_id: serenity::all::GuildId,
        id: sqlx::types::Uuid,
    ) -> Result<Option<Punishment>, crate::Error> {
        let rec = sqlx::query!(
            "SELECT id, src, guild_id, punishment, creator, target, state, handle_log, created_at, duration, reason, data FROM punishments WHERE id = $1 AND guild_id = $2",
            id,
            guild_id.to_string(),
        )
        .fetch_optional(db)
        .await?;

        match rec {
            Some(row) => Ok(Some(Punishment {
                id: row.id,
                src: row.src,
                guild_id: row.guild_id.parse()?,
                punishment: row.punishment,
                creator: PunishmentTarget::from_str(&row.creator)?,
                target: PunishmentTarget::from_str(&row.target)?,
                handle_log: row.handle_log,
                created_at: row.created_at,
                duration: row.duration.map(|d| {
                    let secs = splashcore_rs::utils::pg_interval_to_secs(d);
                    std::time::Duration::from_secs(secs.try_into().unwrap())
                }),
                state: PunishmentState::from_str(&row.state)?,
                reason: row.reason,
                data: row.data,
            })),
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

        let rec = sqlx::query!(
            "SELECT id, src, guild_id, punishment, creator, target, state, handle_log, created_at, duration, reason, data FROM punishments WHERE guild_id = $1 ORDER BY created_at DESC OFFSET $2 LIMIT $3",
            guild_id.to_string(),
            (page as i64 - 1) * PAGE_SIZE,
            PAGE_SIZE,
        )
        .fetch_all(db)
        .await?;

        let mut punishments = Vec::new();

        for row in rec {
            punishments.push(Punishment {
                id: row.id,
                src: row.src,
                guild_id: row.guild_id.parse()?,
                punishment: row.punishment,
                creator: PunishmentTarget::from_str(&row.creator)?,
                target: PunishmentTarget::from_str(&row.target)?,
                handle_log: row.handle_log,
                created_at: row.created_at,
                duration: row.duration.map(|d| {
                    let secs = splashcore_rs::utils::pg_interval_to_secs(d);
                    std::time::Duration::from_secs(secs.try_into().unwrap())
                }),
                state: PunishmentState::from_str(&row.state)?,
                reason: row.reason,
                data: row.data,
            });
        }

        Ok(punishments)
    }

    async fn get_expired(db: impl sqlx::PgExecutor<'_>) -> Result<Vec<Punishment>, crate::Error> {
        let rec = sqlx::query!(
            "SELECT id, src, guild_id, punishment, creator, target, state, handle_log, created_at, duration, reason, data FROM punishments WHERE duration IS NOT NULL AND state = 'active' AND (created_at + duration) < NOW()",
        )
        .fetch_all(db)
        .await?;

        let mut punishments = Vec::new();

        for row in rec {
            punishments.push(Punishment {
                id: row.id,
                src: row.src,
                guild_id: row.guild_id.parse()?,
                punishment: row.punishment,
                creator: PunishmentTarget::from_str(&row.creator)?,
                target: PunishmentTarget::from_str(&row.target)?,
                handle_log: row.handle_log,
                created_at: row.created_at,
                duration: row.duration.map(|d| {
                    let secs = splashcore_rs::utils::pg_interval_to_secs(d);
                    std::time::Duration::from_secs(secs.try_into().unwrap())
                }),
                state: PunishmentState::from_str(&row.state)?,
                reason: row.reason,
                data: row.data,
            });
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
        let ret_data = sqlx::query!(
            r#"
            INSERT INTO punishments (src, guild_id, punishment, creator, target, handle_log, duration, reason, data, state)
            VALUES ($1, $2, $3, $4, $5, $6, make_interval(secs => $7), $8, $9, $10) RETURNING id, created_at
            "#,
            self.src,
            self.guild_id.to_string(),
            self.punishment,
            self.creator.to_string(),
            self.target.to_string(),
            self.handle_log,
            self.duration.map(|d| d.as_secs() as f64),
            self.reason,
            self.data,
            self.state.to_string(),
        )
        .fetch_one(db)
        .await?;

        Ok(self.to_punishment(ret_data.id, ret_data.created_at))
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
