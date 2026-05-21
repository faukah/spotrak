use std::{env, fmt, net::IpAddr, path::PathBuf, time::Duration};

use chrono_tz::Tz;
use secrecy::SecretString;
use thiserror::Error;
use url::Url;

pub const MIN_SPOTIFY_API_DELAY: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub struct Config {
    pub database_url: SecretString,
    pub api_endpoint: Url,
    pub client_endpoint: Url,
    pub spotify_public: String,
    pub spotify_secret: SecretString,
    pub spotify_token_encryption_key: SecretString,
    pub spotify_market: Option<String>,
    pub port: u16,
    pub timezone: Tz,
    pub log_level: String,
    pub cookie_validity: Duration,
    pub spotify_api_delay: Duration,
    pub cors: CorsConfig,
    pub import_dir: PathBuf,
    pub max_import_cache_size: u64,
    pub prometheus_username: Option<String>,
    pub prometheus_password: Option<SecretString>,
}

#[derive(Clone, Debug)]
pub enum CorsConfig {
    Any,
    Origins(Vec<Url>),
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required environment variable {0}")]
    Missing(&'static str),
    #[error("invalid environment variable {name}: {message}")]
    Invalid { name: &'static str, message: String },
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();

        let database_url = SecretString::from(required("DATABASE_URL")?);
        let api_endpoint = parse_url("API_ENDPOINT", &required("API_ENDPOINT")?)?;
        let client_endpoint = parse_url("CLIENT_ENDPOINT", &required("CLIENT_ENDPOINT")?)?;
        let spotify_public = required("SPOTIFY_PUBLIC")?;
        let spotify_secret_raw = required("SPOTIFY_SECRET")?;
        let spotify_token_encryption_key = SecretString::from(
            optional_non_empty("SPOTIFY_TOKEN_ENCRYPTION_KEY")
                .unwrap_or_else(|| spotify_secret_raw.clone()),
        );
        let spotify_secret = SecretString::from(spotify_secret_raw);
        let spotify_market = parse_market(optional_non_empty("SPOTIFY_MARKET"))?;
        let port = parse_or("PORT", 8080)?;
        let timezone = parse_timezone(&env::var("TIMEZONE").unwrap_or_else(|_| "UTC".to_owned()))?;
        let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_owned());
        let cookie_validity = Duration::from_millis(parse_or(
            "COOKIE_VALIDITY_MS",
            30 * 24 * 60 * 60 * 1000_u64,
        )?);
        // Spotify no longer supports several bulk metadata endpoints, so imports may
        // need many single-item requests. Hard-cap throughput at one Spotify request
        // every five seconds; deployments may configure a larger delay, but not a smaller one.
        let spotify_api_delay_ms = parse_or("SPOTIFY_API_DELAY_MS", 5000_u64)?;
        let spotify_api_delay =
            Duration::from_millis(spotify_api_delay_ms).max(MIN_SPOTIFY_API_DELAY);
        let cors_default = client_endpoint.origin().ascii_serialization();
        let cors = expand_cors(
            parse_cors(&env::var("CORS").unwrap_or(cors_default))?,
            &client_endpoint,
        )?;
        let import_dir =
            PathBuf::from(env::var("IMPORT_DIR").unwrap_or_else(|_| "./imports".to_owned()));
        let max_import_cache_size = parse_or("MAX_IMPORT_CACHE_SIZE", 512 * 1024 * 1024_u64)?;
        let prometheus_username = optional_non_empty("PROMETHEUS_USERNAME");
        let prometheus_password = optional_non_empty("PROMETHEUS_PASSWORD").map(SecretString::from);

        validate_http_url("API_ENDPOINT", &api_endpoint)?;
        validate_http_url("CLIENT_ENDPOINT", &client_endpoint)?;
        validate_spotify_redirect_url(&api_endpoint)?;
        if spotify_public.trim().is_empty() {
            return Err(ConfigError::Invalid {
                name: "SPOTIFY_PUBLIC",
                message: "must not be empty".to_owned(),
            });
        }

        Ok(Self {
            database_url,
            api_endpoint,
            client_endpoint,
            spotify_public,
            spotify_secret,
            spotify_token_encryption_key,
            spotify_market,
            port,
            timezone,
            log_level,
            cookie_validity,
            spotify_api_delay,
            cors,
            import_dir,
            max_import_cache_size,
            prometheus_username,
            prometheus_password,
        })
    }

    pub fn oauth_callback_url(&self) -> String {
        self.api_endpoint
            .join("/api/v1/auth/spotify/callback")
            .expect("static callback path is valid")
            .to_string()
    }

    pub fn secure_cookies(&self) -> bool {
        self.api_endpoint.scheme() == "https"
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("database_url", &"<redacted>")
            .field("api_endpoint", &self.api_endpoint)
            .field("client_endpoint", &self.client_endpoint)
            .field("spotify_public", &"<redacted>")
            .field("spotify_secret", &"<redacted>")
            .field("spotify_token_encryption_key", &"<redacted>")
            .field("spotify_market", &self.spotify_market)
            .field("port", &self.port)
            .field("timezone", &self.timezone)
            .field("log_level", &self.log_level)
            .field("cookie_validity", &self.cookie_validity)
            .field("spotify_api_delay", &self.spotify_api_delay)
            .field("cors", &self.cors)
            .field("import_dir", &self.import_dir)
            .field("max_import_cache_size", &self.max_import_cache_size)
            .field(
                "prometheus_username",
                &self.prometheus_username.as_ref().map(|_| "<set>"),
            )
            .field(
                "prometheus_password",
                &self.prometheus_password.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}

fn required(name: &'static str) -> Result<String, ConfigError> {
    optional_non_empty(name).ok_or(ConfigError::Missing(name))
}

fn optional_non_empty(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn parse_url(name: &'static str, value: &str) -> Result<Url, ConfigError> {
    Url::parse(value).map_err(|err| ConfigError::Invalid {
        name,
        message: err.to_string(),
    })
}

fn validate_http_url(name: &'static str, url: &Url) -> Result<(), ConfigError> {
    if url.scheme() == "http" || url.scheme() == "https" {
        return Ok(());
    }
    Err(ConfigError::Invalid {
        name,
        message: "must use http or https".to_owned(),
    })
}

fn validate_spotify_redirect_url(url: &Url) -> Result<(), ConfigError> {
    if url.scheme() == "https" || is_loopback_url(url) {
        return Ok(());
    }
    Err(ConfigError::Invalid {
        name: "API_ENDPOINT",
        message: "Spotify OAuth redirect URI must use https unless the host is localhost/loopback for development".to_owned(),
    })
}

fn is_loopback_url(url: &Url) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

fn parse_or<T>(name: &'static str, default: T) -> Result<T, ConfigError>
where
    T: std::str::FromStr,
    T::Err: fmt::Display,
{
    match env::var(name) {
        Ok(value) => value.parse::<T>().map_err(|err| ConfigError::Invalid {
            name,
            message: err.to_string(),
        }),
        Err(_) => Ok(default),
    }
}

fn parse_market(value: Option<String>) -> Result<Option<String>, ConfigError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let market = value.trim().to_ascii_uppercase();
    if market.len() == 2 && market.chars().all(|ch| ch.is_ascii_uppercase()) {
        Ok(Some(market))
    } else {
        Err(ConfigError::Invalid {
            name: "SPOTIFY_MARKET",
            message: "must be an ISO 3166-1 alpha-2 country code such as US, FR, or DE".to_owned(),
        })
    }
}

fn parse_timezone(value: &str) -> Result<Tz, ConfigError> {
    value.parse::<Tz>().map_err(|err| ConfigError::Invalid {
        name: "TIMEZONE",
        message: err.to_string(),
    })
}

fn parse_cors(value: &str) -> Result<CorsConfig, ConfigError> {
    if value.trim() == "*" {
        return Ok(CorsConfig::Any);
    }

    let mut origins = Vec::new();
    for origin in value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        origins.push(parse_url("CORS", origin)?);
    }

    if origins.is_empty() {
        return Err(ConfigError::Invalid {
            name: "CORS",
            message: "must be '*' or a comma-separated list of origins".to_owned(),
        });
    }

    Ok(CorsConfig::Origins(origins))
}

fn expand_cors(config: CorsConfig, client_endpoint: &Url) -> Result<CorsConfig, ConfigError> {
    let CorsConfig::Origins(mut origins) = config else {
        return Ok(config);
    };

    origins.push(client_endpoint.clone());

    let mut expanded = Vec::new();
    for origin in &origins {
        if let Some(mirror) = loopback_mirror_origin(origin)? {
            expanded.push(mirror);
        }
    }
    origins.extend(expanded);

    let mut seen = std::collections::BTreeSet::new();
    origins.retain(|origin| seen.insert(origin.origin().ascii_serialization()));

    Ok(CorsConfig::Origins(origins))
}

fn loopback_mirror_origin(origin: &Url) -> Result<Option<Url>, ConfigError> {
    let Some(host) = origin.host_str() else {
        return Ok(None);
    };
    let mirror_host = match host {
        "127.0.0.1" => "localhost",
        "localhost" => "127.0.0.1",
        _ => return Ok(None),
    };
    let port = origin
        .port()
        .map(|port| format!(":{port}"))
        .unwrap_or_default();
    let value = format!("{}://{}{}", origin.scheme(), mirror_host, port);
    parse_url("CORS", &value).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cors_any() {
        assert!(matches!(parse_cors("*").unwrap(), CorsConfig::Any));
    }

    #[test]
    fn rejects_invalid_timezone() {
        assert!(parse_timezone("No/SuchZone").is_err());
    }

    #[test]
    fn cors_expansion_adds_loopback_counterpart_and_client_endpoint() {
        let cors = parse_cors("http://127.0.0.1:4322").unwrap();
        let client = Url::parse("http://localhost:4322").unwrap();
        let CorsConfig::Origins(origins) = expand_cors(cors, &client).unwrap() else {
            panic!("expected origins");
        };
        let origins = origins
            .iter()
            .map(|origin| origin.origin().ascii_serialization())
            .collect::<Vec<_>>();
        assert!(origins.contains(&"http://127.0.0.1:4322".to_owned()));
        assert!(origins.contains(&"http://localhost:4322".to_owned()));
    }

    #[test]
    fn spotify_redirect_url_requires_https_except_loopback() {
        assert!(
            validate_spotify_redirect_url(&Url::parse("https://spotrak.example").unwrap()).is_ok()
        );
        assert!(
            validate_spotify_redirect_url(&Url::parse("http://127.0.0.1:8080").unwrap()).is_ok()
        );
        assert!(
            validate_spotify_redirect_url(&Url::parse("http://localhost:8080").unwrap()).is_ok()
        );
        assert!(
            validate_spotify_redirect_url(&Url::parse("http://spotrak.example").unwrap()).is_err()
        );
    }

    #[test]
    fn spotify_delay_minimum_is_five_seconds() {
        assert_eq!(MIN_SPOTIFY_API_DELAY, Duration::from_secs(5));
    }
}
