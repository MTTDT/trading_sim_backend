use axum::{
    extract::{Query, State},
    response::Redirect,
};
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl,
    Scope, TokenResponse, TokenUrl,
};
use serde::Deserialize;
use sqlx::PgPool;
use tower_sessions::Session;
use uuid::Uuid;

use crate::state::{AppError, AppState};

use axum_extra::extract::cookie::CookieJar;
use super::auth::{auth_cookie, issue_jwt};

const STATE_KEY: &str = "oauth_state";

// Env vars:
//   GITHUB_CLIENT_ID, GITHUB_CLIENT_SECRET
//   GITHUB_REDIRECT_URL  e.g. http://localhost:5173/api/auth/github/callback
//   FRONTEND_URL         e.g. http://localhost:5173  (where to send the user afterwards)

fn internal(e: impl std::fmt::Display) -> AppError {
    AppError::Internal(e.to_string())
}

fn env(name: &str) -> Result<String, AppError> {
    std::env::var(name).map_err(|_| AppError::Internal(format!("missing env var {name}")))
}

fn frontend_url() -> String {
    std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".into())
}

fn github_client() -> Result<
    BasicClient<
        oauth2::EndpointSet,    // auth url set
        oauth2::EndpointNotSet, // device auth url
        oauth2::EndpointNotSet, // introspection url
        oauth2::EndpointNotSet, // revocation url
        oauth2::EndpointSet,    // token url set
    >,
    AppError,
> {
    Ok(BasicClient::new(ClientId::new(env("GITHUB_CLIENT_ID")?))
        .set_client_secret(ClientSecret::new(env("GITHUB_CLIENT_SECRET")?))
        .set_auth_uri(
            AuthUrl::new("https://github.com/login/oauth/authorize".into()).map_err(internal)?,
        )
        .set_token_uri(
            TokenUrl::new("https://github.com/login/oauth/access_token".into()).map_err(internal)?,
        )
        .set_redirect_uri(RedirectUrl::new(env("GITHUB_REDIRECT_URL")?).map_err(internal)?))
}

/// Step 1: GET /auth/github -> redirect the browser to GitHub.
pub async fn github_login(session: Session) -> Result<Redirect, AppError> {
    let client = github_client()?;
    let (url, csrf) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("read:user".into()))
        .add_scope(Scope::new("user:email".into())) // needed: the profile email is often private
        .url();

    session.insert(STATE_KEY, csrf.secret().to_string()).await?;
    Ok(Redirect::to(url.as_str()))
}

#[derive(Deserialize)]
pub struct CallbackParams {
    code: String,
    state: String,
}

#[derive(Deserialize)]
struct GithubUser {
    id: i64,
    login: String,
    name: Option<String>,
}

#[derive(Deserialize)]
struct GithubEmail {
    email: String,
    primary: bool,
    verified: bool,
}

/// Step 2: GitHub sends the browser back here with ?code=...&state=...
pub async fn github_callback(
    State(app): State<AppState>,
    session: Session,
    jar: CookieJar,
    Query(params): Query<CallbackParams>,
) -> Result<(CookieJar, Redirect), AppError> {
    // 1. CSRF check: the state we stored must match the one GitHub echoed back.
    let expected: Option<String> = session.remove(STATE_KEY).await?;
    if expected.as_deref() != Some(params.state.as_str()) {
        return Err(AppError::BadRequest("Invalid OAuth state".into()));
    }

    // 2. Exchange the code for an access token.
    let client = github_client()?;
    let http = oauth2::reqwest::ClientBuilder::new()
        .redirect(oauth2::reqwest::redirect::Policy::none())
        .build()?;

    let token = client
        .exchange_code(AuthorizationCode::new(params.code))
        .request_async(&http)
        .await
        .map_err(|e| AppError::Internal(format!("github token exchange failed: {e}")))?;
    let access_token = token.access_token().secret().to_string();

    // 3. Ask GitHub who this is.
    let api = reqwest::Client::new();
    let gh_user: GithubUser = github_get(&api, "https://api.github.com/user", &access_token).await?;
    let gh_emails: Vec<GithubEmail> =
        github_get(&api, "https://api.github.com/user/emails", &access_token).await?;

    // Only trust an email GitHub itself says is primary AND verified.
    let Some(email) = gh_emails
        .into_iter()
        .find(|e| e.primary && e.verified)
        .map(|e| e.email.to_lowercase())
    else {
        return Err(AppError::BadRequest(
            "Your GitHub account has no verified primary email".into(),
        ));
    };
    let name = gh_user.name.unwrap_or(gh_user.login);

    // 4. Find or create our own user, then log them in.
    let user_id = resolve_user(&app.inner.db, &gh_user.id.to_string(), &name, &email).await?;

    let role: String = sqlx::query_scalar("SELECT role FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&app.inner.db)
        .await?;

    let jwt = issue_jwt(user_id, &role)?;
    Ok((jar.add(auth_cookie(jwt)), Redirect::to(&frontend_url())))
}

async fn github_get<T: serde::de::DeserializeOwned>(
    api: &reqwest::Client,
    url: &str,
    token: &str,
) -> Result<T, AppError> {
    let res = api
        .get(url)
        .bearer_auth(token)
        .header("User-Agent", "stock-strategies-app") // GitHub rejects requests without one
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?;
    Ok(res.json::<T>().await?)
}

/// The oauth_accounts table in action:
///  - known (provider, provider_user_id)  -> that user
///  - unknown, but verified email matches -> link the GitHub account to the existing user
///  - otherwise                           -> create a user (no password) and link it
async fn resolve_user(
    db: &PgPool,
    github_id: &str,
    name: &str,
    email: &str,
) -> Result<Uuid, AppError> {
    if let Some((user_id,)) = sqlx::query_as::<_, (Uuid,)>(
        "SELECT user_id FROM oauth_accounts WHERE provider = 'github' AND provider_user_id = $1",
    )
    .bind(github_id)
    .fetch_optional(db)
    .await?
    {
        return Ok(user_id);
    }

    let mut tx = db.begin().await?;

    let existing = sqlx::query_as::<_, (Uuid,)>("SELECT id FROM users WHERE lower(email) = $1")
        .bind(email)
        .fetch_optional(&mut *tx)
        .await?;

    let user_id = match existing {
        Some((id,)) => id,
        None => {
            sqlx::query_as::<_, (Uuid,)>(
                "INSERT INTO users (name, email) VALUES ($1, $2) RETURNING id",
            )
            .bind(name)
            .bind(email)
            .fetch_one(&mut *tx)
            .await?
            .0
        }
    };

    sqlx::query(
        "INSERT INTO oauth_accounts (provider, provider_user_id, user_id)
         VALUES ('github', $1, $2)",
    )
    .bind(github_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(user_id)
}