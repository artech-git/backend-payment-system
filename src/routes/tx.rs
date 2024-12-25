use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{sse::Event, IntoResponse, Sse},
    routing::{get, post},
    Json, Router,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sqlx::{
    types::
        Decimal
    ,
    PgPool,
};
use uuid::Uuid;

use super::{auth::AuthService, utils};

#[derive(Debug, Serialize, Deserialize)]
pub struct Transfer {
    pub sender_id: Uuid,
    pub receiver_id: Uuid,
    pub amount: Decimal,
    pub description: Option<String>,
}

async fn create_transaction(
    headers: HeaderMap,
    State((service, pool)): State<(Arc<AuthService>, PgPool)>,
    Json(transfer): Json<Transfer>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    tracing::info!("Starting transaction creation process");

    let header_uid = match utils::validate_auth_token(headers, &service) {
        Ok(val) => val,
        Err(err) => {
            tracing::error!("Invalid token: {err}");
            return Err((err, "Invalid token"));
        }
    };

    // Transfer sender_id must match the token user_id
    if header_uid != transfer.sender_id {
        tracing::warn!("Unauthorized transaction attempt by user: {header_uid}");
        return Err((axum::http::StatusCode::UNAUTHORIZED, "Invalid token"));
    }

    // Begin a database transaction
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(err) => {
            tracing::error!("Failed to start transaction: {err}");
            return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to transfer amount"));
        }
    };

    let sender_id = transfer.sender_id;
    let receiver_id = transfer.receiver_id;
    let amount = transfer.amount;

    // Deduct amount from sender
    let tx_one = sqlx::query!(
        "UPDATE users SET balance = balance - $1 WHERE id = $2",
        amount,
        sender_id
    )
    .execute(&mut *tx)
    .await;

    // Add amount to receiver
    let tx_two = sqlx::query!(
        "UPDATE users SET balance = balance + $1 WHERE id = $2",
        amount,
        receiver_id
    )
    .execute(&mut *tx)
    .await;

    // Insert transaction record
    let tx_three = sqlx::query!(
        "INSERT INTO transfers (sender_id, recipient_id, amount) VALUES ($1, $2, $3) RETURNING id",
        sender_id,
        receiver_id,
        amount,
    )
    .fetch_one(&mut *tx)
    .await;

    // Validate if all the transactions were successful
    let tx_id = match (tx_one, tx_two, tx_three) {
        (Ok(_), Ok(_), Ok(val)) => val.id,
        _ => {
            tracing::error!("Failed to transfer amount");
            return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to transfer amount"));
        }
    };

    // Commit the transaction
    match tx.commit().await {
        Ok(_) => {
            tracing::info!("Transaction successful with id: {tx_id}");
            return Ok((axum::http::StatusCode::OK, format!("Transaction successful id: {tx_id}")));
        }
        Err(err) => {
            tracing::error!("Failed to commit transaction: {err}");
            return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to transfer amount"));
        }
    }
}

// return a specific transaction by it's transaction_id which belongs to it's user
async fn get_transaction(
    headers: HeaderMap,
    State((service, pool)): State<(Arc<AuthService>, PgPool)>,
    Path(transaction_id): Path<Uuid>, // transaction_id: Uuid
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let header_uid = match utils::validate_auth_token(headers, &service) {
        Ok(val) => val,
        Err(err) => {
            return Err((
                err,
                "Invalid token",
            ));
        }  
    };

    let transaction = match sqlx::query!(
        r#"
        SELECT sender_id, recipient_id, amount FROM transfers WHERE id = $1 AND (sender_id = $2 OR recipient_id = $2)
        "#,
        transaction_id,
        header_uid
    )
    .fetch_one(&pool)
    .await
    {
        Ok(record) => Transfer {
            sender_id: record.sender_id,
            receiver_id: record.recipient_id,
            amount: record.amount,
            description: None,
        },
        Err(err) => {
            tracing::error!("Failed to retrieve transaction: {err}");
            return Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to retrieve transaction",
            ));
        }
    };

    Ok((
        axum::http::StatusCode::OK,
        serde_json::to_string(&transaction).unwrap(),
    ))
}

// return all transactions which a user made through it's user_id 
async fn list_transactions(
    headers: HeaderMap,
    State((service, pool)): State<(Arc<AuthService>, PgPool)>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {

    let user_id = match utils::validate_auth_token(headers, &service) {
        Ok(val) => val, 
        Err(err) =>{
            return Err((err, "Invalid token"));
        }
    };
    
    let cursor = match sqlx::query!(
        "SELECT id, sender_id, recipient_id, amount FROM transfers WHERE sender_id = $1 OR recipient_id = $1",
        user_id
    )
    .fetch_all(&pool) // perhaps this better replaced with fetch method instead but avoided it due to static lifetime bound issue
    .await{
        Ok(cursor) => cursor,
        Err(err) => {
            tracing::error!("Failed to retrieve transactions: {err}");
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to retrieve transactions"));
        }
    };

    let stream = futures::stream::iter(cursor).map(|transaction| {
        let record = transaction;
        let transfer = Transfer {
            sender_id: record.sender_id,
            receiver_id: record.recipient_id,
            amount: record.amount,
            description: None,
        };
        Event::default().json_data(transfer)
    });

    let sse = Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
        .interval(std::time::Duration::from_secs(2))
        .text("keep-alive-text"),
    );

    Ok(sse)
}

pub fn tx_route(service: Arc<AuthService>, pool: PgPool) -> Router {
    Router::new()
        .route("/tx/transfer", post(create_transaction))
        .route("/tx/get_tx/:uid", get(get_transaction))
        .route("/tx/list_txs", get(list_transactions))
        .with_state((service, pool))
}
