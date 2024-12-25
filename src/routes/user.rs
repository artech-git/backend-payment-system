use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use sqlx::{FromRow, PgPool};

use serde::{Deserialize, Serialize};
use sqlx::types::Decimal;
use uuid::Uuid;

use crate::db::user::User;

use super::{auth::AuthService, utils::validate_auth_token};

async fn get_user(
    headers: HeaderMap,
    State((service, pool)): State<(Arc<AuthService>, PgPool)>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let uid = match validate_auth_token(headers, &service) {
        Ok(val) => {
            tracing::info!("Token validation succeeded for user: {}", val);
            val
        }
        Err(err) => {
            tracing::error!("Token validation failed: {:?}", err);
            return Err((err, "Invalid token"));
        }
    };
    let user_id = uid;

    // generate our query
    let mut query_builder = sqlx::QueryBuilder::new("SELECT * FROM users WHERE id = ");
    query_builder.push_bind(user_id);
    let query = query_builder.build();

    let user = query.fetch_one(&pool).await;

    match user.map(|row| User::from_row(&row)) {
        Ok(Ok(user)) => {
            let body = serde_json::to_string(&user).unwrap();
            tracing::info!("User found: {}", user_id);
            return Ok((StatusCode::OK, body));
        }
        _ => {
            tracing::error!("User not found: {}", user_id);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "User not found",
            ));
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUser {
    pub user_id: Uuid,
    #[serde(rename = "name")]
    pub new_name: String,
    #[serde(rename = "email")]
    pub new_email: String,
}

async fn update_user(
    headers: HeaderMap,
    State((service, pool)): State<(Arc<AuthService>, PgPool)>,
    Json(payload): Json<UpdateUser>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let user_id = match validate_auth_token(headers, &service) {
        Ok(val) => val,
        Err(err) => {
            tracing::error!("Token validation failed: {:?}", err);
            return Err((err, "Invalid token"));
        }
    };

    if payload.user_id != user_id {
        tracing::warn!("Unauthorized update attempt by user: {}", user_id);
        return Ok((StatusCode::UNAUTHORIZED, "Unauthorized"));
    }

    let mut query_builder = sqlx::QueryBuilder::new("UPDATE users SET ");
    query_builder
        .push("full_name = ")
        .push_bind(&payload.new_name)
        .push(", email = ")
        .push_bind(&payload.new_email);

    let query = query_builder.build();
    let result = query.execute(&pool).await;

    match result {
        Ok(_) => {
            tracing::info!("User updated successfully: {}", user_id);
            return Ok((StatusCode::OK, "User updated successfully"));
        }
        Err(err) => {
            tracing::error!("Failed to update user: {:?}", err);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update user",
            ));
        }
    }
}

//method for inserting a new user
#[derive(Debug, Serialize, Deserialize)]
pub struct Deposit {
    pub email: String,
    pub full_name: String,
    pub amount: Decimal,
}

async fn deposit(
    headers: HeaderMap,
    State((service, pool)): State<(Arc<AuthService>, PgPool)>,
    Json(payload): Json<Deposit>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let user_id = match validate_auth_token(headers, &service) {
        Ok(val) => val,
        Err(err) => {
            return Err((err, "Invalid token"));
        }
    };

    let user_email = match sqlx::query!("SELECT email FROM users WHERE id = $1", user_id)
        .fetch_one(&pool)
        .await
        .map(|row| row.email)
    {
        Ok(email) => email,
        Err(err) => {
            tracing::error!("Failed to get user email: {err}");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create user",
            ));
        }
    };

    if payload.email != user_email {
        return Err((StatusCode::UNAUTHORIZED, "Unauthorized"));
    }

    //check if user alredy exits
    match service.repo.find_user_by_email(&payload.email).await {
        Ok(Some(_)) => {
            tracing::info!("User discovered in database");
            ()
        }
        Err(err) => {
            tracing::warn!("user not found in database: {err}");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create user",
            ));
        }
        _ => {
            tracing::error!("Failed to create user");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create user",
            ));
        }
    };

    let query = sqlx::query!(
        r#"
        UPDATE users SET balance = balance + $1 WHERE email = $2
        RETURNING id, balance
        "#,
        payload.amount,
        payload.email
    )
    .fetch_one(&pool)
    .await;

    match query {
        Ok(record) => {
            let balance = record.balance.to_string();
            tracing::info!(
                "User balance updated successfully for user: {}. New balance: {balance}",
                record.id
            );
            let body = format!(
                "User balance updated successfully. New balance: {}",
                balance
            );
            Ok((StatusCode::OK, body))
        }
        Err(err) => {
            tracing::info!("Failed to update user balance: {err}");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update user balance",
            ));
        }
    }
}

pub fn user_routes(service: Arc<AuthService>, db_pool: PgPool) -> Router {
    Router::new()
        .route("/users/uid", get(get_user))
        .route("/users/update", put(update_user))
        .route("/users/deposit", post(deposit))
        .with_state((service, db_pool))
}
