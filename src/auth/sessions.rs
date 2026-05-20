use axum::http::HeaderMap;
use cookie::{Cookie, SameSite};

use crate::{config::Config, repositories::sessions::SESSION_COOKIE};

pub const OAUTH_STATE_COOKIE: &str = "ys_oauth_state";

pub fn session_token_from_headers(headers: &HeaderMap) -> Option<String> {
    cookie_value_from_headers(headers, SESSION_COOKIE)
}

pub fn oauth_state_from_headers(headers: &HeaderMap) -> Option<String> {
    cookie_value_from_headers(headers, OAUTH_STATE_COOKIE)
}

pub fn login_cookie(config: &Config, token: &str) -> String {
    Cookie::build((SESSION_COOKIE, token.to_owned()))
        .path("/")
        .http_only(true)
        .secure(config.secure_cookies())
        .same_site(SameSite::Lax)
        .max_age(cookie::time::Duration::milliseconds(cookie_max_age_ms(
            config,
        )))
        .build()
        .to_string()
}

pub fn logout_cookie(config: &Config) -> String {
    expired_cookie(config, SESSION_COOKIE)
}

pub fn oauth_state_cookie(config: &Config, state: &str) -> String {
    Cookie::build((OAUTH_STATE_COOKIE, state.to_owned()))
        .path("/api/v1/auth/spotify/callback")
        .http_only(true)
        .secure(config.secure_cookies())
        .same_site(SameSite::Lax)
        .max_age(cookie::time::Duration::minutes(10))
        .build()
        .to_string()
}

pub fn clear_oauth_state_cookie(config: &Config) -> String {
    Cookie::build((OAUTH_STATE_COOKIE, ""))
        .path("/api/v1/auth/spotify/callback")
        .http_only(true)
        .secure(config.secure_cookies())
        .same_site(SameSite::Lax)
        .max_age(cookie::time::Duration::seconds(0))
        .build()
        .to_string()
}

fn expired_cookie(config: &Config, name: &'static str) -> String {
    Cookie::build((name, ""))
        .path("/")
        .http_only(true)
        .secure(config.secure_cookies())
        .same_site(SameSite::Lax)
        .max_age(cookie::time::Duration::seconds(0))
        .build()
        .to_string()
}

fn cookie_value_from_headers(headers: &HeaderMap, name: &str) -> Option<String> {
    for header in headers.get_all(axum::http::header::COOKIE) {
        let Ok(header) = header.to_str() else {
            continue;
        };
        for part in header.split(';') {
            let trimmed = part.trim();
            if let Ok(cookie) = Cookie::parse(trimmed.to_owned()) {
                if cookie.name() == name {
                    return Some(cookie.value().to_owned());
                }
            }
        }
    }
    None
}

fn cookie_max_age_ms(config: &Config) -> i64 {
    i64::try_from(config.cookie_validity.as_millis()).unwrap_or(i64::MAX)
}
