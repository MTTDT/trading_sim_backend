use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ToSchema)]
pub struct Collection {
    pub id: Uuid,
    pub strategy_id: Uuid,
    pub name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CollectionName {
    pub name: String,
}

impl Collection {
    pub async fn create(db: &PgPool, strategy_id: Uuid, name: &str) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Collection>(
            "INSERT INTO collections (strategy_id, name) VALUES ($1, $2)
             RETURNING id, strategy_id, name",
        )
        .bind(strategy_id)
        .bind(name)
        .fetch_one(db)
        .await
    }

    pub async fn get_for_strategy(db: &PgPool, strategy_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Collection>(
            "SELECT id, strategy_id, name FROM collections WHERE strategy_id = $1 ORDER BY name",
        )
        .bind(strategy_id)
        .fetch_all(db)
        .await
    }

    /// Only returns the collection if it belongs to the given strategy.
    pub async fn get_in_strategy(db: &PgPool, strategy_id: Uuid, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Collection>(
            "SELECT id, strategy_id, name FROM collections WHERE id = $1 AND strategy_id = $2",
        )
        .bind(id)
        .bind(strategy_id)
        .fetch_optional(db)
        .await
    }

    pub async fn rename(db: &PgPool, strategy_id: Uuid, id: Uuid, name: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Collection>(
            "UPDATE collections SET name = $3 WHERE id = $1 AND strategy_id = $2
             RETURNING id, strategy_id, name",
        )
        .bind(id)
        .bind(strategy_id)
        .bind(name)
        .fetch_optional(db)
        .await
    }

    pub async fn delete(db: &PgPool, strategy_id: Uuid, id: Uuid) -> Result<bool, sqlx::Error> {
        let r = sqlx::query("DELETE FROM collections WHERE id = $1 AND strategy_id = $2")
            .bind(id)
            .bind(strategy_id)
            .execute(db)
            .await?;
        Ok(r.rows_affected() > 0)
    }
}