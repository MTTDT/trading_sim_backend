use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{FromRequestParts, State},
    http::{request::Parts, StatusCode},
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tower_sessions::{cookie::time::Duration, Expiry, SessionManagerLayer};
use uuid::Uuid;

use super::oauth;
use crate::session_store::PgSessionStore;
use crate::state::{AppError, AppState};
use crate::user::User;

const MIN_PASSWORD: usize = 8;
const COOKIE_NAME: &str = "token";

#[derive(Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: Uuid,
    pub role: String,
    pub exp: usize,
}

// Session layer: now only used for the OAuth `state` check.
pub async fn session_layer(
    db: PgPool,
) -> Result<SessionManagerLayer<PgSessionStore>, sqlx::Error> {
    let store = PgSessionStore::new(db);
    store.migrate().await?;

    let cleaner = store.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(3600));
        loop {
            tick.tick().await;
            if let Err(e) = cleaner.delete_expired().await {
                tracing::warn!("session cleanup failed: {e}");
            }
        }
    });

    Ok(SessionManagerLayer::new(store)
        .with_secure(!cfg!(debug_assertions))
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::days(7))))
}

// ---------------------------------------------------------------------------
// JWT
// ---------------------------------------------------------------------------

fn jwt_secret() -> Result<String, AppError> {
    std::env::var("JWT_SECRET").map_err(|_| AppError::Internal("missing JWT_SECRET".into()))
}

pub fn issue_jwt(user_id: Uuid, role: &str) -> Result<String, AppError> {
    let exp = (chrono::Utc::now() + chrono::Duration::days(7)).timestamp() as usize;
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &Claims { sub: user_id, role: role.into(), exp },
        &jsonwebtoken::EncodingKey::from_secret(jwt_secret()?.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("jwt encode failed: {e}")))
}

pub fn auth_cookie(token: String) -> Cookie<'static> {
    Cookie::build((COOKIE_NAME, token))
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(!cfg!(debug_assertions))
        .path("/")
        .max_age(Duration::days(7))
        .build()
}

// Extractor: put `CurrentUser` in a handler's arguments to require login.
pub struct CurrentUser(pub Claims);

impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar.get(COOKIE_NAME).ok_or(AppError::Unauthorized)?.value();

        let data = jsonwebtoken::decode::<Claims>(
            token,
            &jsonwebtoken::DecodingKey::from_secret(jwt_secret()?.as_bytes()),
            &jsonwebtoken::Validation::default(), // checks signature and exp
        )
        .map_err(|_| AppError::Unauthorized)?;

        Ok(CurrentUser(data.claims))
    }
}

// ---------------------------------------------------------------------------
// Password helpers
// ---------------------------------------------------------------------------

async fn hash_password(password: String) -> Result<String, AppError> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
    })
    .await
    .map_err(|e| AppError::Internal(format!("hash task failed: {e}")))?
    .map_err(|e| AppError::Internal(format!("hashing failed: {e}")))
}

async fn verify_password(password: String, hash: String) -> bool {
    tokio::task::spawn_blocking(move || {
        PasswordHash::new(&hash)
            .map(|parsed| {
                Argon2::default()
                    .verify_password(password.as_bytes(), &parsed)
                    .is_ok()
            })
            .unwrap_or(false)
    })
    .await
    .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

fn valid_email(email: &str) -> bool {
    let mut parts = email.splitn(2, '@');
    matches!((parts.next(), parts.next()), (Some(l), Some(d)) if !l.is_empty() && d.contains('.'))
}

pub async fn register(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, CookieJar, Json<User>), AppError> {
    let name = req.name.trim().to_string();
    let email = req.email.trim().to_lowercase();

    if name.is_empty() {
        return Err(AppError::Validation("Name is required".into()));
    }
    if !valid_email(&email) {
        return Err(AppError::Validation("Enter a valid email address".into()));
    }
    if req.password.len() < MIN_PASSWORD {
        return Err(AppError::Validation(format!(
            "Password must be at least {MIN_PASSWORD} characters"
        )));
    }

    let hash = hash_password(req.password).await?;

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (name, email, password_hash) VALUES ($1, $2, $3)
         RETURNING id, name, email",
    )
    .bind(&name)
    .bind(&email)
    .bind(&hash)
    .fetch_one(&state.inner.db)
    .await?;

    // New users always get the DB default role, 'user'.
    let jwt = issue_jwt(user.id, "user")?;
    Ok((StatusCode::CREATED, jar.add(auth_cookie(jwt)), Json(user)))
}

#[derive(sqlx::FromRow)]
struct Credentials {
    id: Uuid,
    name: String,
    email: String,
    role: String,
    password_hash: Option<String>,
}

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(req): Json<LoginRequest>,
) -> Result<(CookieJar, Json<User>), AppError> {
    let email = req.email.trim().to_lowercase();

    let row = sqlx::query_as::<_, Credentials>(
        "SELECT id, name, email, role, password_hash FROM users WHERE lower(email) = $1",
    )
    .bind(&email)
    .fetch_optional(&state.inner.db)
    .await?;

    let Some(row) = row else {
        return Err(AppError::Unauthorized);
    };
    let Some(hash) = row.password_hash else {
        return Err(AppError::Unauthorized);
    };
    if !verify_password(req.password, hash).await {
        return Err(AppError::Unauthorized);
    }

    let jwt = issue_jwt(row.id, &row.role)?;
    Ok((
        jar.add(auth_cookie(jwt)),
        Json(User { id: row.id, name: row.name, email: row.email }),
    ))
}

pub async fn logout(jar: CookieJar) -> (CookieJar, StatusCode) {
    // The path must match the one used when the cookie was set.
    let removal = Cookie::build((COOKIE_NAME, "")).path("/").build();
    (jar.remove(removal), StatusCode::NO_CONTENT)
}

pub async fn me(
    State(state): State<AppState>,
    CurrentUser(claims): CurrentUser,
) -> Result<Json<User>, AppError> {
    match User::get_by_id(&state.inner.db, claims.sub).await? {
        Some(user) => Ok(Json(user)),
        None => Err(AppError::Unauthorized),
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/me", get(me))
        .route("/auth/github", get(oauth::github_login))
        .route("/auth/github/callback", get(oauth::github_callback))
}