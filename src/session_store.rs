
use async_trait::async_trait;
use sqlx::PgPool;
use tower_sessions::{
    session::{Id, Record},
    session_store::{self, SessionStore},
};

/// Postgres-backed session store written against your own `sqlx` (0.9),
/// so no separate store crate (which pins sqlx 0.8) is needed.
///
/// Table layout: id (random session id), data (the session as JSON),
/// expires_at (unix seconds). Every query is a static string, so none of
/// them need `AssertSqlSafe`.
#[derive(Clone, Debug)]
pub struct PgSessionStore {
    pool: PgPool,
}

fn backend(e: sqlx::Error) -> session_store::Error {
    session_store::Error::Backend(e.to_string())
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

const UPSERT: &str = "INSERT INTO app_sessions (id, data, expires_at) VALUES ($1, $2, $3)
     ON CONFLICT (id) DO UPDATE SET data = EXCLUDED.data, expires_at = EXCLUDED.expires_at";

impl PgSessionStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Creates the table if it doesn't exist. You can also put this SQL in your own migrations.
    pub async fn migrate(&self) -> sqlx::Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS app_sessions (
                id         TEXT PRIMARY KEY,
                data       TEXT NOT NULL,
                expires_at BIGINT NOT NULL
            )",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS app_sessions_expires_at_idx ON app_sessions (expires_at)")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Removes expired rows. Expired sessions are already ignored on load;
    /// this only keeps the table small.
    pub async fn delete_expired(&self) -> sqlx::Result<u64> {
        let res = sqlx::query("DELETE FROM app_sessions WHERE expires_at < $1")
            .bind(now_unix())
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected())
    }
}

#[async_trait]
impl SessionStore for PgSessionStore {
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        let mut tx = self.pool.begin().await.map_err(backend)?;

        // Ids are random, but make sure we never overwrite someone else's session.
        loop {
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM app_sessions WHERE id = $1)")
                    .bind(record.id.to_string())
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(backend)?;
            if !exists {
                break;
            }
            record.id = Id::default();
        }

        let data = serde_json::to_string(record)
            .map_err(|e| session_store::Error::Encode(e.to_string()))?;
        sqlx::query(UPSERT)
            .bind(record.id.to_string())
            .bind(data)
            .bind(record.expiry_date.unix_timestamp())
            .execute(&mut *tx)
            .await
            .map_err(backend)?;

        tx.commit().await.map_err(backend)?;
        Ok(())
    }

    async fn save(&self, record: &Record) -> session_store::Result<()> {
        let data = serde_json::to_string(record)
            .map_err(|e| session_store::Error::Encode(e.to_string()))?;
        sqlx::query(UPSERT)
            .bind(record.id.to_string())
            .bind(data)
            .bind(record.expiry_date.unix_timestamp())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT data FROM app_sessions WHERE id = $1 AND expires_at > $2")
                .bind(session_id.to_string())
                .bind(now_unix())
                .fetch_optional(&self.pool)
                .await
                .map_err(backend)?;

        match row {
            Some((data,)) => serde_json::from_str(&data)
                .map(Some)
                .map_err(|e| session_store::Error::Decode(e.to_string())),
            None => Ok(None),
        }
    }

    async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
        sqlx::query("DELETE FROM app_sessions WHERE id = $1")
            .bind(session_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }
}