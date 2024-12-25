use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Uuid,
    pub user_id: Uuid,
    pub amount: Decimal,
    pub transaction_type: TransactionType,
    pub status: TransactionStatus,
    pub reference_id: Option<String>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Transfer,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,
    Completed,
    Failed,
}
