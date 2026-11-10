use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;
use crate::filter_request::FilterRequest;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, sqlx::FromRow, ToSchema)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
}


impl User {
    pub async fn create(db: &PgPool, name: &str, email: &str) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING id, name, email",
        )
        .bind(name)
        .bind(email)
        .fetch_one(db)
        .await
    }

   pub async fn get_all(db: &PgPool, f_req: FilterRequest) -> Result<Vec<Self>, sqlx::Error> {
        let (pattern, limit, offset) = f_req.resolve();


        sqlx::query_as::<_, User>(
            r#"SELECT id, name, email FROM users
            WHERE ($1::text IS NULL OR name ILIKE $1)
            ORDER BY name, id LIMIT $2 OFFSET $3"#,
        )
        .bind(pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await
    }


    pub async fn get_by_id(db: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT id, name, email FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(db)
            .await
    }

    pub async fn update(
        db: &PgPool,
        id: Uuid,
        name: Option<&str>,
        email: Option<&str>,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, User>(
            "UPDATE users
             SET name  = COALESCE($2, name),
                 email = COALESCE($3, email)
             WHERE id = $1
             RETURNING id, name, email",
        )
        .bind(id)
        .bind(name)
        .bind(email)
        .fetch_optional(db)
        .await
    }

    pub async fn delete(db: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}