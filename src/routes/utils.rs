use axum::http::{HeaderMap, StatusCode};
use uuid::Uuid;

use super::auth::AuthService;

#[inline]
pub fn validate_auth_token(headers: HeaderMap, service: &AuthService) -> Result<Uuid, StatusCode> {
    let jwt_header_token = match headers.get("Authorization").map(|token| token.to_str()) {
        Some(Ok(token)) => token,
        _ => {
            return Err(StatusCode::UNAUTHORIZED);
        }
    };
    //validate our token
    match service.verify_token(jwt_header_token) {
        Ok(user) => Ok(user),
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}

#[inline]
pub fn check_password(password: &str) -> Result<(), Box<dyn std::error::Error>> {
    if password.len() < 8 {
        return Err("Password must be at least 8 characters".into());
    }
    if !password.chars().any(|c| c.is_uppercase()) {
        return Err("Password must contain at least one uppercase letter".into());
    }
    if !password.chars().any(|c| c.is_lowercase()) {
        return Err("Password must contain at least one lowercase letter".into());
    }
    if !password.chars().any(|c| c.is_digit(10)) {
        return Err("Password must contain at least one digit".into());
    }
    if !password.chars().any(|c| !c.is_alphanumeric()) {
        return Err("Password must contain at least one special character".into());
    }
    Ok(())
}