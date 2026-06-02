use chrono::{Utc, Duration};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use once_cell::sync::Lazy;
use std::env;
use serde::{Deserialize, Serialize};
use common::User;
use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Argon2
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // subject (username)
    pub exp: usize,   // expiration time
    pub role: String, // user role
}

static SECRET_KEY: Lazy<String> = Lazy::new(|| {
    env::var("JWT_SECRET").unwrap_or_else(|_| "your-secret-key-change-in-production".to_string())
});

pub fn create_jwt(user: &User) -> String {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: user.username.clone(),
        exp: expiration as usize,
        role: user.role.clone(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(SECRET_KEY.as_bytes()),
    )
    .expect("JWT creation failed")
}

pub fn validate_jwt(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(SECRET_KEY.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
}

pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2.hash_password(password.as_bytes(), &salt)
        .expect("Password hashing failed")
        .to_string()
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok()
}