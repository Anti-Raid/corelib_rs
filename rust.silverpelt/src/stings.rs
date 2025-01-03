use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Represents a sting on AntiRaid
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sting {
    /// The sting ID
    pub id: sqlx::types::Uuid,
    /// Src of the sting, this can be useful to store the source of a sting
    pub src: Option<String>,
    /// The number of stings
    pub stings: i32,
    /// The reason for the stings (optional)
    pub reason: Option<String>,
    /// The reason the stings were voided
    pub void_reason: Option<String>,
    /// The guild ID the sting targets
    pub guild_id: serenity::all::GuildId,
    /// The creator of the sting
    pub creator: StingTarget,
    /// The target of the sting
    pub target: StingTarget,
    /// The state of the sting
    pub state: StingState,
    /// When the sting was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the sting expires as a chrono duration
    pub duration: Option<std::time::Duration>,
    /// The data/metadata present within the sting, if any
    pub sting_data: Option<serde_json::Value>,
    /// The handle log encountered while handling the sting
    pub handle_log: serde_json::Value,
}

impl Sting {
    /// Returns a sting by ID
    pub async fn get(
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
    pub async fn list(
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

    pub async fn get_expired(db: impl sqlx::PgExecutor<'_>) -> Result<Vec<Sting>, crate::Error> {
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
    pub async fn dispatch_create_event(
        self,
        ctx: serenity::all::Context,
    ) -> Result<(), crate::Error> {
        let guild_id = self.guild_id;
        crate::ar_event::AntiraidEvent::StingCreate(self)
            .dispatch_to_template_worker_and_nowait(&ctx.data::<crate::data::Data>(), guild_id)
            .await?;

        Ok(())
    }

    /// Dispatch a StingUpdate event
    pub async fn dispatch_update_event(
        self,
        ctx: serenity::all::Context,
    ) -> Result<(), crate::Error> {
        let guild_id = self.guild_id;
        crate::ar_event::AntiraidEvent::StingUpdate(self)
            .dispatch_to_template_worker_and_nowait(&ctx.data::<crate::data::Data>(), guild_id)
            .await?;

        Ok(())
    }

    /// Dispatch a StingDelete event
    pub async fn dispatch_delete_event(
        self,
        ctx: serenity::all::Context,
    ) -> Result<(), crate::Error> {
        let guild_id = self.guild_id;
        crate::ar_event::AntiraidEvent::StingDelete(self)
            .dispatch_to_template_worker_and_nowait(&ctx.data::<crate::data::Data>(), guild_id)
            .await?;

        Ok(())
    }

    /// Updates the database from the Sting data
    pub async fn update_without_dispatch(
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
    pub async fn update_and_dispatch(
        self,
        db: impl sqlx::PgExecutor<'_>,
        ctx: serenity::all::Context,
    ) -> Result<(), crate::Error> {
        self.update_without_dispatch(db).await?;

        self.dispatch_update_event(ctx).await?;

        Ok(())
    }

    /// Deletes a sting by ID
    pub async fn delete_without_dispatch(
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
    pub async fn delete_and_dispatch(
        self,
        db: impl sqlx::PgExecutor<'_>,
        ctx: serenity::all::Context,
    ) -> Result<(), crate::Error> {
        Self::delete_without_dispatch(db, self.guild_id, self.id).await?;

        self.dispatch_delete_event(ctx).await?;

        Ok(())
    }
}

/// Data required to create a sting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StingCreate {
    /// Src of the sting, this can be useful to store the source of the sting
    pub src: Option<String>,
    /// The number of stings
    pub stings: i32,
    /// The reason for the stings (optional)
    pub reason: Option<String>,
    /// The reason the stings were voided
    pub void_reason: Option<String>,
    /// The guild ID the sting targets
    pub guild_id: serenity::all::GuildId,
    /// The creator of the sting
    pub creator: StingTarget,
    /// The target of the sting
    pub target: StingTarget,
    /// The state of the sting
    pub state: StingState,
    /// When the sting expires as a chrono duration
    pub duration: Option<std::time::Duration>,
    /// The data/metadata present within the sting, if any
    pub sting_data: Option<serde_json::Value>,
}

impl StingCreate {
    pub fn to_sting(
        self,
        id: sqlx::types::Uuid,
        created_at: chrono::DateTime<chrono::Utc>,
    ) -> Sting {
        Sting {
            id,
            src: self.src,
            stings: self.stings,
            reason: self.reason,
            void_reason: self.void_reason,
            guild_id: self.guild_id,
            creator: self.creator,
            target: self.target,
            state: self.state,
            created_at,
            duration: self.duration,
            sting_data: self.sting_data,
            handle_log: serde_json::Value::Null,
        }
    }

    /// Creates a new Sting without dispatching it as an event
    pub async fn create_without_dispatch(
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
    pub async fn create_and_dispatch(
        self,
        ctx: serenity::all::Context,
        db: impl sqlx::PgExecutor<'_>,
    ) -> Result<(), crate::Error> {
        let sting = self.create_without_dispatch(db).await?;

        sting.dispatch_create_event(ctx).await?;

        Ok(())
    }

    /// Creates a new Sting and dispatches it as an event in one go
    pub async fn create_and_dispatch_returning_id(
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

/// A sting target (either user or system)
#[derive(Debug, Clone, Copy)]
pub enum StingTarget {
    /// The sting was created by a user
    User(serenity::all::UserId),
    /// The sting was created by the system
    System,
}

impl std::fmt::Display for StingTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StingTarget::User(user_id) => write!(f, "user:{}", user_id),
            StingTarget::System => write!(f, "system"),
        }
    }
}

impl std::str::FromStr for StingTarget {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "system" {
            Ok(StingTarget::System)
        } else {
            let user_id = s
                .strip_prefix("user:")
                .ok_or_else(|| format!("Invalid sting creator: {}", s))?;
            Ok(StingTarget::User(
                user_id
                    .parse()
                    .map_err(|e| format!("Invalid user ID: {}", e))?,
            ))
        }
    }
}

// Serde impls for StingTarget
impl Serialize for StingTarget {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&format!("{}", self))
    }
}

impl<'de> Deserialize<'de> for StingTarget {
    fn deserialize<D>(deserializer: D) -> Result<StingTarget, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        StingTarget::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Hash, Default, Debug, Clone, Copy, PartialEq)]
pub enum StingState {
    #[default]
    Active,
    Voided,
    Handled,
}

impl std::fmt::Display for StingState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StingState::Active => write!(f, "active"),
            StingState::Voided => write!(f, "voided"),
            StingState::Handled => write!(f, "handled"),
        }
    }
}

impl std::str::FromStr for StingState {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(StingState::Active),
            "voided" => Ok(StingState::Voided),
            "handled" => Ok(StingState::Handled),
            _ => Err(format!("Invalid sting state: {}", s).into()),
        }
    }
}

// Serde impls for StingState
impl Serialize for StingState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(&format!("{}", self))
    }
}

impl<'de> Deserialize<'de> for StingState {
    fn deserialize<D>(deserializer: D) -> Result<StingState, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        StingState::from_str(&s).map_err(serde::de::Error::custom)
    }
}

pub struct StingAggregate {
    /// Src of the sting, this can be useful if a module wants to store the source of the sting
    pub src: Option<String>,
    /// The target of the sting
    pub target: StingTarget,
    /// The total number of stings matching this aggregate
    pub total_stings: i64,
}

impl StingAggregate {
    /// Returns the sum of all total stings in the aggregate
    pub fn total_stings(vec: Vec<StingAggregate>) -> i64 {
        vec.iter().map(|x| x.total_stings).sum()
    }

    /// Returns the total stings per-user
    ///
    /// Returns (user_id_map, system_stings)
    pub fn total_stings_per_user(
        vec: Vec<StingAggregate>,
    ) -> (std::collections::HashMap<serenity::all::UserId, i64>, i64) {
        let mut map = std::collections::HashMap::new();

        let mut system_stings = 0;

        for sting in vec {
            match sting.target {
                StingTarget::System => {
                    system_stings += sting.total_stings;
                }
                StingTarget::User(user_id) => {
                    *map.entry(user_id).or_insert(0) += sting.total_stings;
                }
            }
        }

        // Add system stings to each user
        for (_, total_stings) in map.iter_mut() {
            *total_stings += system_stings;
        }

        (map, system_stings)
    }
}

/// Returns total stings the user has
pub async fn get_aggregate_stings_for_guild_user(
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

pub async fn get_aggregate_stings_for_guild(
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
