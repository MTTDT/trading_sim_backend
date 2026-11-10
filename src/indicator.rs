use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{AssertSqlSafe, PgPool};
use uuid::Uuid;
use utoipa::ToSchema;

const COLS: &str = "id, user_id, name, description, kind, builtin_key, formula, param_schema";

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ToSchema)]
pub struct Indicator {
    pub id: Uuid,
    pub user_id: Option<Uuid>, // None = built-in
    pub name: String,
    pub description: Option<String>,
    pub kind: String,
    pub builtin_key: Option<String>,
    pub formula: Option<String>,
    pub param_schema: Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct NewIndicator {
    pub user_id: Uuid, // TODO: take from auth instead of the body
    pub name: String,
    pub description: Option<String>,
    pub formula: String,
    pub param_schema: Option<Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateIndicator {
    pub name: Option<String>,
    pub description: Option<String>,
    pub formula: Option<String>,
    pub param_schema: Option<Value>,
}

impl Indicator {
    pub async fn create(db: &PgPool, n: &NewIndicator) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Indicator>(AssertSqlSafe(format!(
            "INSERT INTO indicators (user_id, name, description, kind, formula, param_schema)
             VALUES ($1, $2, $3, 'formula', $4, COALESCE($5, '{{}}'::jsonb))
             RETURNING {COLS}"
        )))
        .bind(n.user_id)
        .bind(&n.name)
        .bind(&n.description)
        .bind(&n.formula)
        .bind(&n.param_schema)
        .fetch_one(db)
        .await
    }

    /// Built-ins, plus the given user's own indicators when `user_id` is Some.
    pub async fn get_visible(db: &PgPool, user_id: Option<Uuid>) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Indicator>(AssertSqlSafe(format!(
            "SELECT {COLS} FROM indicators
             WHERE user_id IS NULL OR user_id = $1
             ORDER BY name"
        )))
        .bind(user_id)
        .fetch_all(db)
        .await
    }

    pub async fn get_by_id(db: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Indicator>(AssertSqlSafe(format!("SELECT {COLS} FROM indicators WHERE id = $1")))
            .bind(id)
            .fetch_optional(db)
            .await
    }

    /// Built-in indicators (user_id IS NULL) can't be modified.
    pub async fn update(
        db: &PgPool,
        id: Uuid,
        u: &UpdateIndicator,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Indicator>(AssertSqlSafe(format!(
            "UPDATE indicators SET
                name         = COALESCE($2, name),
                description  = COALESCE($3, description),
                formula      = COALESCE($4, formula),
                param_schema = COALESCE($5, param_schema)
             WHERE id = $1 AND user_id IS NOT NULL
             RETURNING {COLS}"
        )))
        .bind(id)
        .bind(&u.name)
        .bind(&u.description)
        .bind(&u.formula)
        .bind(&u.param_schema)
        .fetch_optional(db)
        .await
    }

    pub async fn delete(db: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
        let r = sqlx::query("DELETE FROM indicators WHERE id = $1 AND user_id IS NOT NULL")
            .bind(id)
            .execute(db)
            .await?;
        Ok(r.rows_affected() > 0)
    }
}