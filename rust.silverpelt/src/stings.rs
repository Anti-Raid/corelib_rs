use antiraid_types::stings::{Sting, StingAggregate, StingCreate, StingState, StingTarget};
use std::str::FromStr;

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
    async fn dispatch_create_event(self, ctx: serenity::all::Context) -> Result<(), crate::Error>;

    /// Dispatch a StingUpdate event
    async fn dispatch_update_event(self, ctx: serenity::all::Context) -> Result<(), crate::Error>;

    /// Dispatch a StingDelete event
    async fn dispatch_delete_event(self, ctx: serenity::all::Context) -> Result<(), crate::Error>;

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
    ) -> Result<(), crate::Error>;
}

impl StingOperations for Sting {
    /// Returns a sting by ID
    async fn get(
        db: impl sqlx::PgExecutor<'_>,
        guild_id: serenity::all::GuildId,
        id: sqlx::types::Uuid,
    ) -> Result<Option<Sting>, crate::Error> {
        let rec = sqlx::query!(
            "SELECT id, src, stings, reason, void_reason, guild_id, creator, target, state, sting_data, created_at, duration, handle_log FROM stings WHERE id = $1 AND guild_id = $2",
            id,
            guild_id.to_string(),
        )
        .fetch_optional(db)
        .await?;

        match rec {
            Some(row) => Ok(Some(Sting {
                id: row.id,
                src: row.src,
                stings: row.stings,
                reason: row.reason,
                void_reason: row.void_reason,
                guild_id: row.guild_id.parse()?,
                creator: StingTarget::from_str(&row.creator)?,
                target: StingTarget::from_str(&row.target)?,
                state: StingState::from_str(&row.state)?,
                sting_data: row.sting_data,
                created_at: row.created_at,
                duration: row.duration.map(|d| {
                    let secs = splashcore_rs::utils::pg_interval_to_secs(d);
                    std::time::Duration::from_secs(secs.try_into().unwrap())
                }),
                handle_log: row.handle_log,
            })),
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

        let rec = sqlx::query!(
            "SELECT id, src, stings, reason, void_reason, guild_id, creator, target, state, sting_data, created_at, duration, handle_log FROM stings WHERE guild_id = $1 ORDER BY created_at DESC OFFSET $2 LIMIT $3",
            guild_id.to_string(),
            (page - 1) * PAGE_SIZE,
            PAGE_SIZE,
        )
        .fetch_all(db)
        .await?;

        let mut stings = Vec::new();

        for row in rec {
            stings.push(Sting {
                id: row.id,
                src: row.src,
                stings: row.stings,
                reason: row.reason,
                void_reason: row.void_reason,
                guild_id: row.guild_id.parse()?,
                creator: StingTarget::from_str(&row.creator)?,
                target: StingTarget::from_str(&row.target)?,
                state: StingState::from_str(&row.state)?,
                sting_data: row.sting_data,
                created_at: row.created_at,
                duration: row.duration.map(|d| {
                    let secs = splashcore_rs::utils::pg_interval_to_secs(d);
                    std::time::Duration::from_secs(secs.try_into().unwrap())
                }),
                handle_log: row.handle_log,
            });
        }

        Ok(stings)
    }

    async fn get_expired(db: impl sqlx::PgExecutor<'_>) -> Result<Vec<Sting>, crate::Error> {
        let rec = sqlx::query!(
            "SELECT id, src, stings, reason, void_reason, guild_id, creator, target, state, sting_data, created_at, duration, handle_log FROM stings WHERE duration IS NOT NULL AND state = 'active' AND (created_at + duration) < NOW()",
        )
        .fetch_all(db)
        .await?;

        let mut stings = Vec::new();

        for row in rec {
            stings.push(Sting {
                id: row.id,
                src: row.src,
                stings: row.stings,
                reason: row.reason,
                void_reason: row.void_reason,
                guild_id: row.guild_id.parse()?,
                creator: StingTarget::from_str(&row.creator)?,
                target: StingTarget::from_str(&row.target)?,
                state: StingState::from_str(&row.state)?,
                sting_data: row.sting_data,
                created_at: row.created_at,
                duration: row.duration.map(|d| {
                    let secs = splashcore_rs::utils::pg_interval_to_secs(d);
                    std::time::Duration::from_secs(secs.try_into().unwrap())
                }),
                handle_log: row.handle_log,
            });
        }

        Ok(stings)
    }

    /// Dispatch a StingCreate event
    async fn dispatch_create_event(self, ctx: serenity::all::Context) -> Result<(), crate::Error> {
        let guild_id = self.guild_id;
        crate::ar_event::AntiraidEvent::StingCreate(self)
            .dispatch_to_template_worker_and_nowait(&ctx.data::<crate::data::Data>(), guild_id)
            .await?;

        Ok(())
    }

    /// Dispatch a StingUpdate event
    async fn dispatch_update_event(self, ctx: serenity::all::Context) -> Result<(), crate::Error> {
        let guild_id = self.guild_id;
        crate::ar_event::AntiraidEvent::StingUpdate(self)
            .dispatch_to_template_worker_and_nowait(&ctx.data::<crate::data::Data>(), guild_id)
            .await?;

        Ok(())
    }

    /// Dispatch a StingDelete event
    async fn dispatch_delete_event(self, ctx: serenity::all::Context) -> Result<(), crate::Error> {
        let guild_id = self.guild_id;
        crate::ar_event::AntiraidEvent::StingDelete(self)
            .dispatch_to_template_worker_and_nowait(&ctx.data::<crate::data::Data>(), guild_id)
            .await?;

        Ok(())
    }

    /// Returns the guild ID associated with a sting
    async fn guild_id(
        id: sqlx::types::Uuid,
        db: impl sqlx::PgExecutor<'_>,
    ) -> Result<serenity::all::GuildId, crate::Error> {
        let guild_id = sqlx::query!("SELECT guild_id FROM stings WHERE id = $1", id)
            .fetch_one(db)
            .await
            .map_err(|_| "Sting not found")?
            .guild_id;

        Ok(guild_id.parse()?)
    }

    /// Updates the database from the Sting data
    async fn update_without_dispatch(
        &self,
        db: impl sqlx::PgExecutor<'_>,
    ) -> Result<(), crate::Error> {
        sqlx::query!(
            "UPDATE stings SET src = $1, stings = $2, reason = $3, void_reason = $4, creator = $5, target = $6, state = $7, duration = make_interval(secs => $8), sting_data = $9, handle_log = $10 WHERE id = $11 AND guild_id = $12",
            self.src,
            self.stings,
            self.reason,
            self.void_reason,
            self.creator.to_string(),
            self.target.to_string(),
            self.state.to_string(),
            self.duration.map(|d| d.as_secs() as f64),
            self.sting_data,
            self.handle_log,
            self.id,
            self.guild_id.to_string(),
        )
        .execute(db)
        .await?;

        Ok(())
    }

    /// Updates the sting and dispatches a StingUpdate event
    async fn update_and_dispatch(
        self,
        db: impl sqlx::PgExecutor<'_>,
        ctx: serenity::all::Context,
    ) -> Result<(), crate::Error> {
        self.update_without_dispatch(db).await?;

        self.dispatch_update_event(ctx).await?;

        Ok(())
    }

    /// Deletes a sting by ID
    async fn delete_without_dispatch(
        db: impl sqlx::PgExecutor<'_>,
        guild_id: serenity::all::GuildId,
        id: sqlx::types::Uuid,
    ) -> Result<(), crate::Error> {
        sqlx::query!(
            "DELETE FROM stings WHERE id = $1 AND guild_id = $2",
            id,
            guild_id.to_string(),
        )
        .execute(db)
        .await?;

        Ok(())
    }

    /// Deletes a sting by ID and dispatches a StingDelete event
    async fn delete_and_dispatch(
        self,
        db: impl sqlx::PgExecutor<'_>,
        ctx: serenity::all::Context,
    ) -> Result<(), crate::Error> {
        Self::delete_without_dispatch(db, self.guild_id, self.id).await?;

        self.dispatch_delete_event(ctx).await?;

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
    ) -> Result<(), crate::Error>;

    /// Creates a new Sting and dispatches it as an event in one go
    async fn create_and_dispatch_returning_id(
        self,
        ctx: serenity::all::Context,
        db: impl sqlx::PgExecutor<'_>,
    ) -> Result<sqlx::types::Uuid, crate::Error>;
}

impl StingCreateOperations for StingCreate {
    /// Creates a new Sting without dispatching it as an event
    async fn create_without_dispatch(
        self,
        db: impl sqlx::PgExecutor<'_>,
    ) -> Result<Sting, crate::Error> {
        let ret_data = sqlx::query!(
            r#"
            INSERT INTO stings (src, stings, reason, void_reason, guild_id, target, creator, state, duration, sting_data)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, make_interval(secs => $9), $10) RETURNING id, created_at
            "#,
            self.src,
            self.stings,
            self.reason,
            self.void_reason,
            self.guild_id.to_string(),
            self.target.to_string(),
            self.creator.to_string(),
            self.state.to_string(),
            self.duration.map(|d| d.as_secs() as f64),
            self.sting_data,
        )
        .fetch_one(db)
        .await?;

        Ok(self.to_sting(ret_data.id, ret_data.created_at))
    }

    /// Creates a new Sting and dispatches it as an event in one go
    async fn create_and_dispatch(
        self,
        ctx: serenity::all::Context,
        db: impl sqlx::PgExecutor<'_>,
    ) -> Result<(), crate::Error> {
        let sting = self.create_without_dispatch(db).await?;

        sting.dispatch_create_event(ctx).await?;

        Ok(())
    }

    /// Creates a new Sting and dispatches it as an event in one go
    async fn create_and_dispatch_returning_id(
        self,
        ctx: serenity::all::Context,
        db: impl sqlx::PgExecutor<'_>,
    ) -> Result<sqlx::types::Uuid, crate::Error> {
        let sting = self.create_without_dispatch(db).await?;
        let sid = sting.id;

        sting.dispatch_create_event(ctx).await?;

        Ok(sid)
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
        let rec = sqlx::query!(
        "SELECT COUNT(*) AS total_stings, src, target FROM stings WHERE guild_id = $1 AND state = 'active' AND (target = $2 OR target = 'system') GROUP BY src, target",
        guild_id.to_string(),
        StingTarget::User(target).to_string(),
    )
    .fetch_all(db)
    .await?;

        let mut stings = Vec::new();

        for row in rec {
            stings.push(StingAggregate {
                src: row.src,
                target: StingTarget::from_str(&row.target)?,
                total_stings: row.total_stings.unwrap_or_default(),
            });
        }

        Ok(stings)
    }

    async fn guild(
        db: impl sqlx::PgExecutor<'_>,
        guild_id: serenity::all::GuildId,
    ) -> Result<Vec<StingAggregate>, crate::Error> {
        let rec = sqlx::query!(
        "SELECT SUM(stings) AS total_stings, src, target FROM stings WHERE guild_id = $1 AND state = 'active' GROUP BY src, target",
        guild_id.to_string(),
    )
    .fetch_all(db)
    .await?;

        let mut stings = Vec::new();

        for row in rec {
            stings.push(StingAggregate {
                src: row.src,
                target: StingTarget::from_str(&row.target)?,
                total_stings: row.total_stings.unwrap_or_default(),
            });
        }

        Ok(stings)
    }
}
