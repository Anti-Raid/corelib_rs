use antiraid_types::stings::{Sting, StingAggregate, StingCreate, StingState, StingTarget};
use sqlx::postgres::types::PgInterval;
use sqlx::Row;
use std::str::FromStr;

use crate::{
    ar_event::{AntiraidEventOperations, DispatchEventData},
    pginterval::pg_interval_to_secs,
};

#[allow(async_fn_in_trait)]
pub trait StingOperations: Send + Sync {
    /// Returns a sting by ID
    async fn get(
        db: impl sqlx::PgExecutor<'_>,
        guild_id: serenity::all::GuildId,
        id: sqlx::types::Uuid,
    ) -> Result<Option<Sting>, crate::Error>;

    /// Lists stings for a guild paginated based on page number
    async fn list(
        db: impl sqlx::PgExecutor<'_>,
        guild_id: serenity::all::GuildId,
        page: usize,
    ) -> Result<Vec<Sting>, crate::Error>;

    /// Returns the expired stings
    async fn get_expired(db: impl sqlx::PgExecutor<'_>) -> Result<Vec<Sting>, crate::Error>;

    /// Dispatch a StingCreate event
    async fn dispatch_create_event(
        self,
        ctx: serenity::all::Context,
        dispatch_event_data: &DispatchEventData,
    ) -> Result<(), crate::Error>;

    /// Dispatch a StingUpdate event
    async fn dispatch_update_event(
        self,
        ctx: serenity::all::Context,
        dispatch_event_data: &DispatchEventData,
    ) -> Result<(), crate::Error>;

    /// Dispatch a StingDelete event
    async fn dispatch_delete_event(
        self,
        ctx: serenity::all::Context,
        dispatch_event_data: &DispatchEventData,
    ) -> Result<(), crate::Error>;

    async fn guild_id(
        id: sqlx::types::Uuid,
        db: impl sqlx::PgExecutor<'_>,
    ) -> Result<serenity::all::GuildId, crate::Error>;

    /// Updates the database from the Sting data
    async fn update_without_dispatch(
        &self,
        db: impl sqlx::PgExecutor<'_>,
    ) -> Result<(), crate::Error>;

    /// Updates the sting and dispatches a StingUpdate event
    async fn update_and_dispatch(
        self,
        db: impl sqlx::PgExecutor<'_>,
        ctx: serenity::all::Context,
        dispatch_event_data: &DispatchEventData,
    ) -> Result<(), crate::Error>;

    /// Deletes a sting by ID
    async fn delete_without_dispatch(
        db: impl sqlx::PgExecutor<'_>,
        guild_id: serenity::all::GuildId,
        id: sqlx::types::Uuid,
    ) -> Result<(), crate::Error>;

    /// Deletes a sting by ID and dispatches a StingDelete event
    async fn delete_and_dispatch(
        self,
        db: impl sqlx::PgExecutor<'_>,
        ctx: serenity::all::Context,
        dispatch_event_data: &DispatchEventData,
    ) -> Result<(), crate::Error>;
}

#[derive(sqlx::FromRow)]
struct StingRow {
    id: uuid::Uuid,
    src: Option<String>,
    stings: i32,
    reason: Option<String>,
    void_reason: Option<String>,
    guild_id: String,
    creator: String,
    target: String,
    state: String,
    sting_data: Option<serde_json::Value>,
    created_at: chrono::DateTime<chrono::Utc>,
    duration: Option<PgInterval>,
    handle_log: serde_json::Value,
}

impl StingRow {
    fn into_sting(self) -> Result<Sting, crate::Error> {
        Ok(Sting {
            id: self.id,
            src: self.src,
            stings: self.stings,
            reason: self.reason,
            void_reason: self.void_reason,
            guild_id: self.guild_id.parse()?,
            creator: StingTarget::from_str(&self.creator)?,
            target: StingTarget::from_str(&self.target)?,
            state: StingState::from_str(&self.state)?,
            sting_data: self.sting_data,
            created_at: self.created_at,
            duration: match self.duration {
                Some(d) => {
                    let secs = pg_interval_to_secs(d);
                    Some(std::time::Duration::from_secs(secs.try_into()?))
                }
                None => None,
            },
            handle_log: self.handle_log,
        })
    }
}

impl StingOperations for Sting {
    /// Returns a sting by ID
    async fn get(
        db: impl sqlx::PgExecutor<'_>,
        guild_id: serenity::all::GuildId,
        id: sqlx::types::Uuid,
    ) -> Result<Option<Sting>, crate::Error> {
        let rec: Option<StingRow> = sqlx::query_as(
            "SELECT id, src, stings, reason, void_reason, guild_id, creator, target, state, sting_data, created_at, duration, handle_log FROM stings WHERE id = $1 AND guild_id = $2",
        )
        .bind(id)
        .bind(guild_id.to_string())
        .fetch_optional(db)
        .await?;

        match rec {
            Some(row) => Ok(Some(row.into_sting()?)),
            None => Ok(None),
        }
    }

    /// Lists stings for a guild paginated based on page number
    async fn list(
        db: impl sqlx::PgExecutor<'_>,
        guild_id: serenity::all::GuildId,
        page: usize,
    ) -> Result<Vec<Sting>, crate::Error> {
        const PAGE_SIZE: i64 = 20; // 20 stings per page

        if page > i64::MAX as usize {
            return Err("Page number too large".into());
        }

        let page = std::cmp::max(page, 1) as i64; // Avoid negative pages

        let rec: Vec<StingRow> = sqlx::query_as(
            "SELECT id, src, stings, reason, void_reason, guild_id, creator, target, state, sting_data, created_at, duration, handle_log FROM stings WHERE guild_id = $1 ORDER BY created_at DESC OFFSET $2 LIMIT $3",
        )
        .bind(guild_id.to_string())
        .bind((page - 1) * PAGE_SIZE)
        .bind(PAGE_SIZE)
        .fetch_all(db)
        .await?;

        let mut stings = Vec::new();

        for row in rec {
            stings.push(row.into_sting()?);
        }

        Ok(stings)
    }

    async fn get_expired(db: impl sqlx::PgExecutor<'_>) -> Result<Vec<Sting>, crate::Error> {
        let rec: Vec<StingRow> = sqlx::query_as(
            "SELECT id, src, stings, reason, void_reason, guild_id, creator, target, state, sting_data, created_at, duration, handle_log FROM stings WHERE duration IS NOT NULL AND state = 'active' AND (created_at + duration) < NOW()",
        )
        .fetch_all(db)
        .await?;

        let mut stings = Vec::new();

        for row in rec {
            stings.push(row.into_sting()?);
        }

        Ok(stings)
    }

    /// Dispatch a StingCreate event
    async fn dispatch_create_event(
        self,
        _ctx: serenity::all::Context,
        _dispatch_event_data: &DispatchEventData,
    ) -> Result<(), crate::Error> {
        Ok(()) // disabled as builtins+stings are being rewritten in luau
    }

    /// Dispatch a StingUpdate event
    async fn dispatch_update_event(
        self,
        _ctx: serenity::all::Context,
        _dispatch_event_data: &DispatchEventData,
    ) -> Result<(), crate::Error> {
        Ok(()) // disabled as builtins+stings are being rewritten in luau
    }

    /// Dispatch a StingDelete event
    async fn dispatch_delete_event(
        self,
        _ctx: serenity::all::Context,
        _dispatch_event_data: &DispatchEventData,
    ) -> Result<(), crate::Error> {
        Ok(()) // disabled as builtins+stings are being rewritten in luau
    }

    /// Returns the guild ID associated with a sting
    async fn guild_id(
        id: sqlx::types::Uuid,
        db: impl sqlx::PgExecutor<'_>,
    ) -> Result<serenity::all::GuildId, crate::Error> {
        let guild_id = sqlx::query("SELECT guild_id FROM stings WHERE id = $1")
            .bind(id)
            .fetch_one(db)
            .await
            .map_err(|_| "Sting not found")?
            .try_get::<String, _>("guild_id")?;

        Ok(guild_id.parse()?)
    }

    /// Updates the database from the Sting data
    async fn update_without_dispatch(
        &self,
        db: impl sqlx::PgExecutor<'_>,
    ) -> Result<(), crate::Error> {
        sqlx::query(
            "UPDATE stings SET src = $1, stings = $2, reason = $3, void_reason = $4, creator = $5, target = $6, state = $7, duration = make_interval(secs => $8), sting_data = $9, handle_log = $10 WHERE id = $11 AND guild_id = $12",
        )
        .bind(&self.src)
        .bind(self.stings)
        .bind(&self.reason)
        .bind(&self.void_reason)
        .bind(self.creator.to_string())
        .bind(self.target.to_string())
        .bind(self.state.to_string())
        .bind(self.duration.map(|d| d.as_secs() as f64))
        .bind(&self.sting_data)
        .bind(&self.handle_log)
        .bind(self.id)
        .bind(self.guild_id.to_string())
        .execute(db)
        .await?;

        Ok(())
    }

    /// Updates the sting and dispatches a StingUpdate event
    async fn update_and_dispatch(
        self,
        db: impl sqlx::PgExecutor<'_>,
        ctx: serenity::all::Context,
        dispatch_event_data: &DispatchEventData,
    ) -> Result<(), crate::Error> {
        self.update_without_dispatch(db).await?;

        self.dispatch_update_event(ctx, dispatch_event_data).await?;

        Ok(())
    }

    /// Deletes a sting by ID
    async fn delete_without_dispatch(
        db: impl sqlx::PgExecutor<'_>,
        guild_id: serenity::all::GuildId,
        id: sqlx::types::Uuid,
    ) -> Result<(), crate::Error> {
        sqlx::query("DELETE FROM stings WHERE id = $1 AND guild_id = $2")
            .bind(id)
            .bind(guild_id.to_string())
            .execute(db)
            .await?;

        Ok(())
    }

    /// Deletes a sting by ID and dispatches a StingDelete event
    async fn delete_and_dispatch(
        self,
        db: impl sqlx::PgExecutor<'_>,
        ctx: serenity::all::Context,
        dispatch_event_data: &DispatchEventData,
    ) -> Result<(), crate::Error> {
        Self::delete_without_dispatch(db, self.guild_id, self.id).await?;

        self.dispatch_delete_event(ctx, dispatch_event_data).await?;

        Ok(())
    }
}

#[allow(async_fn_in_trait)]
pub trait StingCreateOperations: Send + Sync {
    /// Creates a new Sting without dispatching it as an event
    async fn create_without_dispatch(
        self,
        db: impl sqlx::PgExecutor<'_>,
    ) -> Result<Sting, crate::Error>;

    /// Creates a new Sting and dispatches it as an event in one go
    async fn create_and_dispatch(
        self,
        ctx: serenity::all::Context,
        db: impl sqlx::PgExecutor<'_>,
        dispatch_event_data: &DispatchEventData,
    ) -> Result<(), crate::Error>;

    /// Creates a new Sting and dispatches it as an event in one go
    async fn create_and_dispatch_returning_id(
        self,
        ctx: serenity::all::Context,
        db: impl sqlx::PgExecutor<'_>,
        dispatch_event_data: &DispatchEventData,
    ) -> Result<sqlx::types::Uuid, crate::Error>;
}

impl StingCreateOperations for StingCreate {
    /// Creates a new Sting without dispatching it as an event
    async fn create_without_dispatch(
        self,
        db: impl sqlx::PgExecutor<'_>,
    ) -> Result<Sting, crate::Error> {
        let ret_data = sqlx::query(
            r#"
            INSERT INTO stings (src, stings, reason, void_reason, guild_id, target, creator, state, duration, sting_data)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, make_interval(secs => $9), $10) RETURNING id, created_at
            "#,
        )
        .bind(&self.src)
        .bind(self.stings)
        .bind(&self.reason)
        .bind(&self.void_reason)
        .bind(self.guild_id.to_string())
        .bind(self.target.to_string())
        .bind(self.creator.to_string())
        .bind(self.state.to_string())
        .bind(self.duration.map(|d| d.as_secs() as f64))
        .bind(&self.sting_data)
        .fetch_one(db)
        .await?;

        Ok(self.to_sting(ret_data.try_get("id")?, ret_data.try_get("created_at")?))
    }

    /// Creates a new Sting and dispatches it as an event in one go
    async fn create_and_dispatch(
        self,
        ctx: serenity::all::Context,
        db: impl sqlx::PgExecutor<'_>,
        dispatch_event_data: &DispatchEventData,
    ) -> Result<(), crate::Error> {
        let sting = self.create_without_dispatch(db).await?;

        sting
            .dispatch_create_event(ctx, dispatch_event_data)
            .await?;

        Ok(())
    }

    /// Creates a new Sting and dispatches it as an event in one go
    async fn create_and_dispatch_returning_id(
        self,
        ctx: serenity::all::Context,
        db: impl sqlx::PgExecutor<'_>,
        dispatch_event_data: &DispatchEventData,
    ) -> Result<sqlx::types::Uuid, crate::Error> {
        let sting = self.create_without_dispatch(db).await?;
        let sid = sting.id;

        sting
            .dispatch_create_event(ctx, dispatch_event_data)
            .await?;

        Ok(sid)
    }
}

#[derive(sqlx::FromRow)]
struct StingAggregateRow {
    src: Option<String>,
    target: String,
    total_stings: Option<i64>,
}

impl StingAggregateRow {
    fn into_sting_aggregate(self) -> Result<StingAggregate, crate::Error> {
        Ok(StingAggregate {
            src: self.src,
            target: StingTarget::from_str(&self.target)?,
            total_stings: self.total_stings.unwrap_or_default(),
        })
    }
}

#[allow(async_fn_in_trait)]
pub trait StingAggregateOperations: Send + Sync {
    /// Returns a StingAggregate set for a user in a guild
    async fn guild_user(
        db: impl sqlx::PgExecutor<'_>,
        guild_id: serenity::all::GuildId,
        target: serenity::all::UserId,
    ) -> Result<Vec<StingAggregate>, crate::Error>;

    /// Returns a StingAggregate set for a guild
    async fn guild(
        db: impl sqlx::PgExecutor<'_>,
        guild_id: serenity::all::GuildId,
    ) -> Result<Vec<StingAggregate>, crate::Error>;
}

impl StingAggregateOperations for StingAggregate {
    async fn guild_user(
        db: impl sqlx::PgExecutor<'_>,
        guild_id: serenity::all::GuildId,
        target: serenity::all::UserId,
    ) -> Result<Vec<StingAggregate>, crate::Error> {
        let rec: Vec<StingAggregateRow> = sqlx::query_as(
        "SELECT COUNT(*) AS total_stings, src, target FROM stings WHERE guild_id = $1 AND state = 'active' AND (target = $2 OR target = 'system') GROUP BY src, target",
        )
        .bind(guild_id.to_string())
        .bind(StingTarget::User(target).to_string())
        .fetch_all(db)
        .await?;

        let mut stings = Vec::new();

        for row in rec {
            stings.push(row.into_sting_aggregate()?);
        }

        Ok(stings)
    }

    async fn guild(
        db: impl sqlx::PgExecutor<'_>,
        guild_id: serenity::all::GuildId,
    ) -> Result<Vec<StingAggregate>, crate::Error> {
        let rec: Vec<StingAggregateRow> = sqlx::query_as(
        "SELECT SUM(stings) AS total_stings, src, target FROM stings WHERE guild_id = $1 AND state = 'active' GROUP BY src, target",
        )
        .bind(guild_id.to_string())
        .fetch_all(db)
        .await?;

        let mut stings = Vec::new();

        for row in rec {
            stings.push(row.into_sting_aggregate()?);
        }

        Ok(stings)
    }
}
