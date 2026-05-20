use axum::http::HeaderMap;
use cookie::{Cookie, SameSite};

use crate::{config::Config, repositories::sessions::SESSION_COOKIE};

pub fn session_token_from_headers(headers: &HeaderMap) -> Option<String> {
    let header = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    for part in header.split(';') {
        let trimmed = part.trim();
        if let Ok(cookie) = Cookie::parse(trimmed.to_owned()) {
            if cookie.name() == SESSION_COOKIE {
                return Some(cookie.value().to_owned());
            }
        }
    }
    None
}

pub fn login_cookie(config: &Config, token: &str) -> String {
    Cookie::build((SESSION_COOKIE, token.to_owned()))
        .path("/")
        .http_only(true)
        .secure(config.secure_cookies())
        .same_site(SameSite::Lax)
        .max_age(cookie::time::Duration::milliseconds(
            config.cookie_validity.as_millis() as i64,
        ))
        .build()
        .to_string()
}

pub fn logout_cookie(config: &Config) -> String {
    Cookie::build((SESSION_COOKIE, ""))
        .path("/")
        .http_only(true)
        .secure(config.secure_cookies())
        .same_site(SameSite::Lax)
        .max_age(cookie::time::Duration::seconds(0))
        .build()
        .to_string()
}
