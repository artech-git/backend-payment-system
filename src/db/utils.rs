use chrono::{DateTime, Utc};
use sqlx::types::time::OffsetDateTime;

pub fn convert_offsetdt_to_dt(offset_datetime: OffsetDateTime) -> DateTime<Utc> {
    // Get the Unix timestamp from OffsetDateTime
    let timestamp = offset_datetime.unix_timestamp();

    // Convert the timestamp into a DateTime<Utc>
    DateTime::<Utc>::from_timestamp_micros(timestamp).unwrap_or(Utc::now())
}
