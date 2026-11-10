use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{AssertSqlSafe, PgPool};
use uuid::Uuid;
use utoipa::ToSchema;

const COLS: &str = "id, strategy_id, indicator_id, role, params, weight::float8 AS weight, position";

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ToSchema)]
pub struct StrategyIndicator {
    pub id: Uuid,
    pub strategy_id: Uuid,
    pub indicator_id: Uuid,
    pub role: String, // entry | exit | filter
    pub params: Value,
    pub weight: f64,
    pub position: i32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct NewStrategyIndicator {
    pub indicator_id: Uuid,
    pub role: Option<String>,
    pub params: Option<Value>,
    pub weight: Option<f64>,
    pub position: Option<i32>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateStrategyIndicator {
    pub role: Option<String>,
    pub params: Option<Value>,
    pub weight: Option<f64>,
    pub position: Option<i32>,
}

impl StrategyIndicator {
    pub async fn add(
        db: &PgPool,
        strategy_id: Uuid,
        n: &NewStrategyIndicator,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, StrategyIndicator>(AssertSqlSafe(format!(
            "INSERT INTO strategy_indicators (strategy_id, indicator_id, role, params, weight, position)
             VALUES ($1, $2, COALESCE($3, 'entry'), COALESCE($4, '{{}}'::jsonb),
                     COALESCE($5::numeric, 1), COALESCE($6, 0))
             RETURNING {COLS}"
        )))
        .bind(strategy_id)
        .bind(n.indicator_id)
        .bind(&n.role)
        .bind(&n.params)
        .bind(n.weight)
        .bind(n.position)
        .fetch_one(db)
        .await
    }

    pub async fn get_for_strategy(db: &PgPool, strategy_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, StrategyIndicator>(AssertSqlSafe(format!(
            "SELECT {COLS} FROM strategy_indicators
             WHERE strategy_id = $1 ORDER BY position"
        )))
        .bind(strategy_id)
        .fetch_all(db)
        .await
    }

    pub async fn get_by_id(db: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, StrategyIndicator>(AssertSqlSafe(format!(
            "SELECT {COLS} FROM strategy_indicators WHERE id = $1"
        )))
        .bind(id)
        .fetch_optional(db)
        .await
    }

    pub async fn update(
        db: &PgPool,
        id: Uuid,
        u: &UpdateStrategyIndicator,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, StrategyIndicator>(AssertSqlSafe(format!(
            "UPDATE strategy_indicators SET
                role     = COALESCE($2, role),
                params   = COALESCE($3, params),
                weight   = COALESCE($4::numeric, weight),
                position = COALESCE($5, position)
             WHERE id = $1
             RETURNING {COLS}"
        )))
        .bind(id)
        .bind(&u.role)
        .bind(&u.params)
        .bind(u.weight)
        .bind(u.position)
        .fetch_optional(db)
        .await
    }

    pub async fn remove(db: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
        let r = sqlx::query("DELETE FROM strategy_indicators WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;
        Ok(r.rows_affected() > 0)
    }
}