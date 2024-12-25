use std::sync::Arc;
use std::time::Duration;

use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2, PasswordHash, PasswordVerifier,
};
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use serde_email::Email;
use sqlx::types::chrono::Utc;
use uuid::Uuid;

use crate::db::auth::AuthRepository;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    sub: Uuid, // user_id
    exp: i64,  // expiration timestamp
    iat: i64,  // issued at timestamp
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    email: Email,
    password: String,
    full_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    email: Email,
    password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    access_token: String,
    refresh_token: String,
    user_uid: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    refresh_token: String,
}

// Authentication service
pub struct AuthService {
    pub repo: AuthRepository,
    jwt_secret: String,
}

impl AuthService {
    pub fn new(repo: AuthRepository, jwt_secret: String) -> Self {
        Self { repo, jwt_secret }
    }

    pub async fn register(
        &self,
        req: RegisterRequest,
    ) -> Result<AuthResponse, Box<dyn std::error::Error>> {
        // Check if user already exists
        if let Some(_) = self.repo.find_user_by_email(req.email.as_str()).await? {
            return Err("User already exists".into());
        }

        //check for password validity
        crate::routes::utils::check_password(&req.password)?;

        // Hash password
        let salt = SaltString::generate(&mut rand::thread_rng());
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(req.password.as_bytes(), &salt)
            .map_err(|_err| "unable to hash password")?
            .to_string();

        // Create user
        let (user, email) = self
            .repo
            .create_user(req.email.as_str(), &password_hash, req.full_name.as_deref())
            .await?;
        tracing::info!("user created with email: {}", email);
        // Generate tokens
        let (access_token, refresh_token) = self.generate_tokens(user)?;

        // Store refresh token
        let expires_at = Utc::now() + Duration::from_secs(60 * 60); // 1 hr
        self.repo
            .store_refresh_token(user, &refresh_token, expires_at)
            .await?;
        tracing::info!("stored refresh token for user: {}", email);
        Ok(AuthResponse {
            access_token,
            refresh_token,
            user_uid: user,
        })
    }

    pub async fn login(
        &self,
        req: LoginRequest,
    ) -> Result<AuthResponse, Box<dyn std::error::Error>> {
        tracing::info!("Attempting to log in user with email: {}", req.email);

        // Find user
        let (user, email, password) = self
            .repo
            .find_user_by_email(req.email.as_str())
            .await?
            .ok_or("Invalid credentials")?;
        tracing::info!("User found with email: {}", email);

        // Verify password
        let parsed_hash =
            PasswordHash::new(&password).map_err(|_err| "unable to generate password")?;
        if !Argon2::default()
            .verify_password(req.password.as_bytes(), &parsed_hash)
            .is_ok()
        {
            tracing::warn!("Invalid credentials for user: {}", email);
            return Err("Invalid credentials".into());
        }
        tracing::info!("Password verified for user: {}", email);

        // Generate tokens
        let (access_token, refresh_token) = self.generate_tokens(user)?;
        tracing::info!("Generated tokens for user: {}", email);

        // Store refresh token
        let expires_at = Utc::now() + Duration::from_secs(60 * 60); // 1 hr
        self.repo
            .store_refresh_token(user, &refresh_token, expires_at)
            .await?;
        tracing::info!("Stored refresh token for user: {}", email);

        Ok(AuthResponse {
            access_token,
            refresh_token,
            user_uid: user,
        })
    }

    pub fn verify_token(&self, token: &str) -> Result<Uuid, Box<dyn std::error::Error>> {
        let mut validation = jsonwebtoken::Validation::default();

        validation.leeway = 10;
        validation.validate_exp = true;
        validation.algorithms = vec![jsonwebtoken::Algorithm::HS256];

        let token_data = jsonwebtoken::decode::<Claims>(
            token,
            &jsonwebtoken::DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation,
        )
        .map_err(|err| {
            tracing::error!("Error decoding token: {:?}", err);
            "Invalid token"
        })?;

        Ok(token_data.claims.sub)
    }

    pub async fn refresh_token(
        &self,
        refresh_token: String,
    ) -> Result<AuthResponse, Box<dyn std::error::Error>> {
        // Verify refresh token and get user
        let user = self
            .repo
            .verify_refresh_token(&refresh_token)
            .await?
            .ok_or("Invalid refresh token")?;

        // Generate new tokens
        let (access_token, new_refresh_token) = self.generate_tokens(user.id)?;

        // Store new refresh token
        let expires_at = Utc::now() + Duration::from_secs(60 * 60); // 1 hr
        self.repo
            .store_refresh_token(user.id, &new_refresh_token, expires_at)
            .await?;

        Ok(AuthResponse {
            access_token,
            refresh_token: new_refresh_token,
            user_uid: user.id,
        })
    }

    fn generate_tokens(
        &self,
        user_id: Uuid,
    ) -> Result<(String, String), Box<dyn std::error::Error>> {
        let now = Utc::now();

        // Access token (15 minutes)
        let access_claims = Claims {
            sub: user_id,
            exp: (now + Duration::from_secs(15 * 60)).timestamp(),
            iat: now.timestamp(),
        };

        let access_token = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &access_claims,
            &jsonwebtoken::EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )?;

        // Refresh token
        let refresh_token = Uuid::new_v4().to_string();

        Ok((access_token, refresh_token))
    }

    

}

// Route for handling new user registration
pub async fn register_handler(
    State(service): State<Arc<AuthService>>,
    Json(req): Json<RegisterRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match service.register(req).await {
        Ok(response) => Ok((StatusCode::CREATED, Json(response))),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

// Route for handling user login
pub async fn login_handler(
    State(service): State<Arc<AuthService>>,
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match service.login(req).await {
        Ok(response) => Ok((StatusCode::OK, Json(response))),
        Err(e) => Err((StatusCode::UNAUTHORIZED, e.to_string())),
    }
}

// Route for handling token refresh
pub async fn refresh_token_handler(
    State(service): State<Arc<AuthService>>,
    Json(req): Json<RefreshTokenRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match service.refresh_token(req.refresh_token).await {
        Ok(response) => Ok((StatusCode::OK, Json(response))),
        Err(e) => Err((StatusCode::UNAUTHORIZED, e.to_string())),
    }
}

pub fn auth_routes(service: Arc<AuthService>) -> Router {
    Router::new()
        .route("/auth/register", post(register_handler))
        .route("/auth/login", post(login_handler))
        .route("/auth/refresh", post(refresh_token_handler))
        .with_state(service)
}
