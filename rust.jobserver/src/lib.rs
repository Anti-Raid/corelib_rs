pub mod embed;
pub mod poll;
pub mod spawn;

use indexmap::IndexMap;
use silverpelt::objectstore::ObjectStore;
use sqlx::postgres::types::PgInterval;
use sqlx::PgPool;
use std::str::FromStr;
use std::time::Duration;
use uuid::Uuid;

pub type Error = Box<dyn std::error::Error + Send + Sync>; // This is constant and should be copy pasted

#[derive(serde::Deserialize, serde::Serialize)]
pub struct SpawnResponse {
    pub id: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Spawn {
    pub name: String,
    pub data: serde_json::Value,
    pub create: bool,
    pub execute: bool,
    pub id: Option<String>, // If create is false, this is required
    pub user_id: String,
}

/// Rust internal/special type to better serialize/speed up embed creation
#[derive(serde::Deserialize, serde::Serialize, Clone, PartialEq)]
pub struct Statuses {
    pub level: String,
    pub msg: String,
    pub ts: f64,
    #[serde(rename = "botDisplayIgnore")]
    pub bot_display_ignore: Option<Vec<String>>,

    #[serde(flatten)]
    pub extra_info: IndexMap<String, serde_json::Value>,
}

pub struct Job {
    pub id: Uuid,
    pub name: String,
    pub output: Option<Output>,
    pub fields: IndexMap<String, serde_json::Value>,
    pub statuses: Vec<Statuses>,
    pub guild_id: serenity::all::GuildId,
    pub expiry: Option<chrono::Duration>,
    pub state: String,
    pub resumable: bool,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct Owner {
    pub id: String,
    pub target_type: String,
}

impl FromStr for Owner {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.splitn(2, '/');
        let target_type = split.next().ok_or("Invalid owner.target_type")?;
        let id = split.next().ok_or("Invalid owner.id")?;

        Ok(Self {
            id: id.to_string(),
            target_type: target_type.to_string(),
        })
    }
}

impl From<String> for Owner {
    fn from(s: String) -> Self {
        Self::from_str(&s).unwrap()
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct Output {
    pub filename: String,
}

/// JobCreateResponse is the response upon creation of a job
#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct JobCreateResponse {
    /// The ID of the newly created task
    pub id: String,
}

/// Internal representation of a job in postgres
#[derive(sqlx::FromRow)]
struct JobRow {
    id: Uuid,
    name: String,
    output: Option<serde_json::Value>,
    fields: serde_json::Value,
    statuses: Vec<serde_json::Value>,
    guild_id: String,
    expiry: Option<PgInterval>,
    state: String,
    created_at: chrono::NaiveDateTime,
    resumable: bool,
}

impl Job {
    fn from_pgrow(rec: JobRow) -> Result<Self, Error> {
        let mut statuses = Vec::with_capacity(rec.statuses.len());

        for status in &rec.statuses {
            let status = serde_json::from_value::<Statuses>(status.clone())?;
            statuses.push(status);
        }

        let task = Job {
            id: rec.id,
            name: rec.name,
            output: rec
                .output
                .map(serde_json::from_value::<Output>)
                .transpose()?,
            fields: serde_json::from_value::<IndexMap<String, serde_json::Value>>(rec.fields)?,
            statuses,
            guild_id: rec.guild_id.parse()?,
            expiry: {
                if let Some(expiry) = rec.expiry {
                    let t = expiry.microseconds
                        + 60 * 1_000_000
                        + (expiry.days as i64) * 24 * 60 * 60 * 1_000_000
                        + (expiry.months as i64) * 30 * 24 * 60 * 60 * 1_000_000;
                    Some(chrono::Duration::microseconds(t))
                } else {
                    None
                }
            },
            state: rec.state,
            created_at: rec.created_at,
            resumable: rec.resumable,
        };

        Ok(task)
    }

    /// Fetches a task from the database based on id
    pub async fn from_id(id: Uuid, pool: &PgPool) -> Result<Self, Error> {
        let rec = sqlx::query_as(
            "SELECT id, name, output, statuses, guild_id, expiry, state, created_at, fields, resumable FROM jobs WHERE id = $1 ORDER BY created_at DESC",
        )
        .bind(id)
        .fetch_one(pool)
        .await?;

        Self::from_pgrow(rec)
    }

    /// Fetches all jobs of a guild given guild id
    #[allow(dead_code)] // Will be used in the near future
    pub async fn from_guild(
        guild_id: serenity::all::GuildId,
        pool: &sqlx::PgPool,
    ) -> Result<Vec<Self>, Error> {
        let recs = sqlx::query_as(
            "SELECT id, name, output, statuses, expiry, state, created_at, fields, resumable FROM jobs WHERE guild_id = $1",
        )
        .bind(guild_id.to_string())
        .fetch_all(pool)
        .await?;

        let mut jobs = Vec::new();

        for rec in recs {
            jobs.push(Self::from_pgrow(rec)?);
        }

        Ok(jobs)
    }

    /// Returns all jobs with a specific guild ID and a specific task name
    pub async fn from_guild_and_name(
        guild_id: serenity::all::GuildId,
        name: &str,
        pool: &sqlx::PgPool,
    ) -> Result<Vec<Self>, Error> {
        let recs = sqlx::query_as(
            "SELECT id, name, output, statuses, guild_id, expiry, state, created_at, fields, resumable FROM jobs WHERE guild_id = $1 AND name = $2",
        )
        .bind(guild_id.to_string())
        .bind(name)
        .fetch_all(pool)
        .await?;

        let mut jobs = Vec::new();

        for rec in recs {
            jobs.push(Self::from_pgrow(rec)?);
        }

        Ok(jobs)
    }

    pub fn get_path(&self) -> String {
        format!("jobs/{}", self.id)
    }

    pub fn get_file_path(&self) -> Option<String> {
        let path = self.get_path();

        self.output
            .as_ref()
            .map(|output| format!("{}/{}", path, output.filename))
    }

    #[allow(dead_code)]
    pub async fn get_url(&self, object_store: &ObjectStore) -> Result<String, Error> {
        // Check if the job has an output
        let Some(path) = &self.get_file_path() else {
            return Err("Job has no output".into());
        };

        object_store
            .get_url("antiraid", path, Duration::from_secs(600))
            .await
    }

    /// Deletes the job from the object storage
    pub async fn delete_from_storage(&self, object_store: &ObjectStore) -> Result<(), Error> {
        // Check if the job has an output
        let path = self.get_path();

        let Some(outp) = &self.output else {
            return Err("Job has no output".into());
        };

        object_store
            .delete("antiraid", &format!("{}/{}", path, outp.filename))
            .await?;

        Ok(())
    }

    /// Delete the job from the database, this also consumes the job dropping it from memory
    pub async fn delete_from_db(self, pool: &PgPool) -> Result<(), Error> {
        sqlx::query("DELETE FROM jobs WHERE id = $1")
            .bind(self.id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Deletes the job entirely, this includes deleting it from the object storage and the database
    /// This also consumes the job dropping it from memory
    #[allow(dead_code)] // Will be used in the near future
    pub async fn delete(self, pool: &PgPool, object_store: &ObjectStore) -> Result<(), Error> {
        self.delete_from_storage(object_store).await?;
        self.delete_from_db(pool).await?;

        Ok(())
    }
}
