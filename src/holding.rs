use serde::{Deserialize, Serialize};
use sqlx::{AssertSqlSafe, PgPool};
use uuid::Uuid;

const SELECT: &str = "SELECT h.id, h.collection_id, h.stock_id, s.symbol, s.name,
        h.quantity::float8 AS quantity, h.notes, h.created_at::text AS created_at
    FROM holdings h JOIN stocks s ON s.id = h.stock_id";

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Holding {
    pub id: Uuid,
    pub collection_id: Uuid,
    pub stock_id: Uuid,
    pub symbol: String,
    pub name: String,
    pub quantity: f64,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct NewHolding {
    pub stock_id: Uuid,
    pub quantity: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateHolding {
    pub quantity: Option<f64>,
    pub notes: Option<String>,
}

impl Holding {
    pub async fn list(db: &PgPool, collection_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Holding>(AssertSqlSafe(format!(
            "{SELECT} WHERE h.collection_id = $1 ORDER BY s.symbol"
        )))
        .bind(collection_id)
        .fetch_all(db)
        .await
    }

    pub async fn get(db: &PgPool, collection_id: Uuid, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Holding>(AssertSqlSafe(format!(
            "{SELECT} WHERE h.id = $1 AND h.collection_id = $2"
        )))
        .bind(id)
        .bind(collection_id)
        .fetch_optional(db)
        .await
    }

    pub async fn add(db: &PgPool, collection_id: Uuid, n: &NewHolding) -> Result<Option<Self>, sqlx::Error> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "INSERT INTO holdings (collection_id, stock_id, quantity, notes)
             VALUES ($1, $2, COALESCE($3::numeric, 0), $4)
             ON CONFLICT (collection_id, stock_id) DO NOTHING
             RETURNING id",
        )
        .bind(collection_id)
        .bind(n.stock_id)
        .bind(n.quantity)
        .bind(&n.notes)
        .fetch_optional(db)
        .await?;

        match row {
            Some((id,)) => Self::get(db, collection_id, id).await,
            None => Ok(None),
        }
    }

    pub async fn update(db: &PgPool, collection_id: Uuid, id: Uuid, u: &UpdateHolding) -> Result<Option<Self>, sqlx::Error> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            "UPDATE holdings SET
                quantity = COALESCE($3::numeric, quantity),
                notes    = COALESCE($4, notes)
             WHERE id = $1 AND collection_id = $2
             RETURNING id",
        )
        .bind(id)
        .bind(collection_id)
        .bind(u.quantity)
        .bind(&u.notes)
        .fetch_optional(db)
        .await?;

        match row {
            Some((id,)) => Self::get(db, collection_id, id).await,
            None => Ok(None),
        }
    }

    pub async fn delete(db: &PgPool, collection_id: Uuid, id: Uuid) -> Result<bool, sqlx::Error> {
        let r = sqlx::query("DELETE FROM holdings WHERE id = $1 AND collection_id = $2")
            .bind(id)
            .bind(collection_id)
            .execute(db)
            .await?;
        Ok(r.rows_affected() > 0)
    }
}