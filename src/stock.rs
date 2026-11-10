use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ToSchema)]
pub struct Stock {
    pub id: Uuid,
    pub symbol: String,
    pub name: String,
    pub exchange: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct NewStock {
    pub symbol: String,
    pub name: String,
    pub exchange: Option<String>,
}

#[derive(Debug, Deserialize,  Serialize, ToSchema)]
pub struct FullStock {
    pub symbol: String,
    pub name: String,
    pub close_prices: Vec<f64>,
    pub open_prices: Vec<f64>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateStock {
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub exchange: Option<String>,
}

impl Stock {
    pub async fn create(db: &PgPool, n: &NewStock) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Stock>(
            "INSERT INTO stocks (symbol, name, exchange)
             VALUES (upper($1), $2, COALESCE($3, ''))
             RETURNING id, symbol, name, exchange",
        )
        .bind(&n.symbol)
        .bind(&n.name)
        .bind(&n.exchange)
        .fetch_one(db)
        .await
    }

    pub async fn get_all(db: &PgPool, q: Option<&str>) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Stock>(
            "SELECT id, symbol, name, exchange FROM stocks
             WHERE $1::text IS NULL
                OR symbol ILIKE $1 || '%'
                OR name   ILIKE '%' || $1 || '%'
             ORDER BY symbol
             LIMIT 100",
        )
        .bind(q)
        .fetch_all(db)
        .await
    }

    pub async fn get_by_id(db: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Stock>("SELECT id, symbol, name, exchange FROM stocks WHERE id = $1")
            .bind(id)
            .fetch_optional(db)
            .await
    }

    pub async fn update(
        db: &PgPool,
        id: Uuid,
        u: &UpdateStock,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Stock>(
            "UPDATE stocks SET
                symbol   = COALESCE(upper($2), symbol),
                name     = COALESCE($3, name),
                exchange = COALESCE($4, exchange)
             WHERE id = $1
             RETURNING id, symbol, name, exchange",
        )
        .bind(id)
        .bind(&u.symbol)
        .bind(&u.name)
        .bind(&u.exchange)
        .fetch_optional(db)
        .await
    }

    pub async fn delete(db: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
        let r = sqlx::query("DELETE FROM stocks WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;
        Ok(r.rows_affected() > 0)
    }
}