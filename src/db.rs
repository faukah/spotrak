use std::{
    future::Future,
    marker::PhantomData,
    sync::{Arc, Once},
    time::Instant,
};

use chrono::{DateTime, Utc};
use deadpool_postgres::{
    Client as DeadpoolClient, Config as PoolConfig, ManagerConfig, Pool, RecyclingMethod, Runtime,
    Transaction as DeadpoolTransaction,
};
use rustls::{
    DigitallySignedStruct, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    pki_types::{CertificateDer, ServerName, UnixTime},
};
use serde_json::Value;
use sha2::{Digest, Sha384};
use spotrak_codegen::client::GenericClient;
use tokio_postgres::{
    NoTls, Row,
    types::{FromSqlOwned, ToSql},
};
use tokio_postgres_rustls::MakeRustlsConnect;
use url::Url;
use uuid::Uuid;

use crate::error::{AppError, Result};

pub type Transaction<'a> = DeadpoolTransaction<'a>;
pub type PgPool = Pool;

const MIGRATION_LOCK_KEY: i64 = 7_764_786_970_021;

struct Migration {
    version: i64,
    description: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        description: "initial",
        sql: include_str!("../migrations/0001_initial.sql"),
    },
    Migration {
        version: 2,
        description: "link_import_events",
        sql: include_str!("../migrations/0002_link_import_events.sql"),
    },
    Migration {
        version: 3,
        description: "runtime_and_spotify_cache",
        sql: include_str!("../migrations/0003_runtime_and_spotify_cache.sql"),
    },
    Migration {
        version: 5,
        description: "hour_format",
        sql: include_str!("../migrations/0005_hour_format.sql"),
    },
    Migration {
        version: 6,
        description: "response_cache_and_spotify_queue",
        sql: include_str!("../migrations/0006_response_cache_and_spotify_queue.sql"),
    },
    Migration {
        version: 7,
        description: "oauth_pkce",
        sql: include_str!("../migrations/0007_oauth_pkce.sql"),
    },
    Migration {
        version: 8,
        description: "security_and_queue_hardening",
        sql: include_str!("../migrations/0008_security_and_queue_hardening.sql"),
    },
];

pub fn build_pool(database_url: &str, max_size: usize) -> Result<Pool> {
    let mut config = PoolConfig::new();
    config.url = Some(database_url.to_owned());
    config.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });
    config.pool = Some(deadpool_postgres::PoolConfig {
        max_size,
        ..Default::default()
    });
    match tls_mode(database_url) {
        TlsMode::Disable => config
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .map_err(|err| AppError::internal(err.to_string())),
        mode => config
            .create_pool(Some(Runtime::Tokio1), tls_connector(mode))
            .map_err(|err| AppError::internal(err.to_string())),
    }
}

pub async fn connect_once(database_url: &str) -> Result<tokio_postgres::Client> {
    match tls_mode(database_url) {
        TlsMode::Disable => {
            let (client, connection) = tokio_postgres::connect(database_url, NoTls).await?;
            spawn_connection(connection);
            Ok(client)
        }
        mode => {
            let (client, connection) =
                tokio_postgres::connect(database_url, tls_connector(mode)).await?;
            spawn_connection(connection);
            Ok(client)
        }
    }
}

fn spawn_connection(
    connection: impl Future<Output = std::result::Result<(), tokio_postgres::Error>> + Send + 'static,
) {
    tokio::spawn(async move {
        if let Err(err) = connection.await {
            tracing::error!(?err, "postgres connection task ended with error");
        }
    });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TlsMode {
    Disable,
    Unverified,
    VerifyFull,
}

fn tls_mode(database_url: &str) -> TlsMode {
    let Some(sslmode) = Url::parse(database_url)
        .ok()
        .and_then(|url| sslmode_from_query(url.query()))
    else {
        return TlsMode::Unverified;
    };

    match sslmode.to_ascii_lowercase().as_str() {
        "disable" => TlsMode::Disable,
        "verify-ca" | "verify-full" => TlsMode::VerifyFull,
        _ => TlsMode::Unverified,
    }
}

fn sslmode_from_query(query: Option<&str>) -> Option<String> {
    url::form_urlencoded::parse(query?.as_bytes())
        .find(|(key, _)| key == "sslmode")
        .map(|(_, value)| value.into_owned())
}

fn tls_connector(mode: TlsMode) -> MakeRustlsConnect {
    static TLS_PROVIDER: Once = Once::new();
    TLS_PROVIDER.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });

    let builder = rustls::ClientConfig::builder();
    let config = match mode {
        TlsMode::Disable => unreachable!("disabled TLS should use NoTls"),
        TlsMode::Unverified => builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoCertificateVerification))
            .with_no_client_auth(),
        TlsMode::VerifyFull => {
            let roots =
                rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            builder.with_root_certificates(roots).with_no_client_auth()
        }
    };
    MakeRustlsConnect::new(config)
}

#[derive(Debug)]
struct NoCertificateVerification;

impl ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::ED25519,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
        ]
    }
}

pub async fn migrate(pool: &Pool) -> Result<()> {
    let mut client = pool.get().await?;
    client
        .batch_execute(
            r#"
            CREATE TABLE IF NOT EXISTS _sqlx_migrations (
                version BIGINT PRIMARY KEY,
                description TEXT NOT NULL,
                installed_on TIMESTAMPTZ NOT NULL DEFAULT now(),
                success BOOLEAN NOT NULL,
                checksum BYTEA NOT NULL,
                execution_time BIGINT NOT NULL
            )
            "#,
        )
        .await?;
    GenericClient::execute(
        &client,
        "SELECT pg_advisory_lock($1)",
        &[&MIGRATION_LOCK_KEY],
    )
    .await?;

    let result = migrate_locked(&mut client).await;
    let unlock_result = GenericClient::execute(
        &client,
        "SELECT pg_advisory_unlock($1)",
        &[&MIGRATION_LOCK_KEY],
    )
    .await;
    result?;
    unlock_result?;
    Ok(())
}

async fn migrate_locked(client: &mut DeadpoolClient) -> Result<()> {
    for migration in MIGRATIONS {
        let checksum = Sha384::digest(migration.sql.as_bytes()).to_vec();
        let existing = GenericClient::query_opt(
            &*client,
            "SELECT checksum, success FROM _sqlx_migrations WHERE version = $1",
            &[&migration.version],
        )
        .await?;
        if let Some(row) = existing {
            let stored_checksum: Vec<u8> = row.get("checksum");
            let success: bool = row.get("success");
            if !success {
                return Err(AppError::internal(format!(
                    "migration {} previously failed",
                    migration.version
                )));
            }
            if stored_checksum != checksum {
                return Err(AppError::internal(format!(
                    "migration {} checksum mismatch",
                    migration.version
                )));
            }
            continue;
        }

        apply_migration(client, migration, checksum).await?;
    }

    Ok(())
}

async fn apply_migration(
    client: &mut DeadpoolClient,
    migration: &Migration,
    checksum: Vec<u8>,
) -> Result<()> {
    let started_at = Instant::now();
    if migration_without_transaction(migration.sql) {
        client.batch_execute(migration.sql).await?;
        record_migration(&*client, migration, &checksum, started_at).await?;
    } else {
        let tx = client.transaction().await?;
        tx.batch_execute(migration.sql).await?;
        record_migration(&tx, migration, &checksum, started_at).await?;
        tx.commit().await?;
    }

    tracing::info!(
        version = migration.version,
        description = migration.description,
        "applied database migration"
    );
    Ok(())
}

async fn record_migration(
    client: &impl GenericClient,
    migration: &Migration,
    checksum: &[u8],
    started_at: Instant,
) -> Result<()> {
    let execution_time = started_at.elapsed().as_nanos() as i64;
    let params: [&(dyn ToSql + Sync); 4] = [
        &migration.version,
        &migration.description,
        &checksum,
        &execution_time,
    ];
    client
        .execute(
            r#"
        INSERT INTO _sqlx_migrations
          (version, description, success, checksum, execution_time)
        VALUES ($1, $2, TRUE, $3, $4)
        "#,
            &params,
        )
        .await?;
    Ok(())
}

fn migration_without_transaction(sql: &str) -> bool {
    sql.lines()
        .map(str::trim)
        .take_while(|line| line.is_empty() || line.starts_with("--"))
        .any(|line| line.eq_ignore_ascii_case("-- no-transaction"))
}

pub trait FromPgRow: Sized {
    fn from_pg_row(row: &Row) -> Self;
}

#[macro_export]
macro_rules! impl_from_pg_row {
    ($type:ty { $($field:ident),+ $(,)? }) => {
        impl $crate::db::FromPgRow for $type {
            fn from_pg_row(row: &tokio_postgres::Row) -> Self {
                Self {
                    $($field: row.get(stringify!($field))),+
                }
            }
        }
    };
}

pub struct Query {
    sql: String,
    params: Vec<Box<dyn ToSql + Sync + Send>>,
}

pub struct QueryAs<T> {
    sql: String,
    params: Vec<Box<dyn ToSql + Sync + Send>>,
    _marker: PhantomData<T>,
}

pub struct QueryScalar<T> {
    sql: String,
    params: Vec<Box<dyn ToSql + Sync + Send>>,
    _marker: PhantomData<T>,
}

pub struct QueryResult {
    rows_affected: u64,
}

impl QueryResult {
    pub fn rows_affected(&self) -> u64 {
        self.rows_affected
    }
}

pub fn query(sql: impl AsRef<str>) -> Query {
    Query {
        sql: sql.as_ref().to_owned(),
        params: Vec::new(),
    }
}

pub fn query_as<T>(sql: impl AsRef<str>) -> QueryAs<T> {
    QueryAs {
        sql: sql.as_ref().to_owned(),
        params: Vec::new(),
        _marker: PhantomData,
    }
}

pub fn query_scalar<T>(sql: impl AsRef<str>) -> QueryScalar<T> {
    QueryScalar {
        sql: sql.as_ref().to_owned(),
        params: Vec::new(),
        _marker: PhantomData,
    }
}

impl Query {
    pub fn bind(mut self, value: impl IntoSqlParam) -> Self {
        self.params.push(value.into_sql_param());
        self
    }

    pub async fn execute(self, executor: impl DbExecutor) -> Result<QueryResult> {
        let params = param_refs(&self.params);
        let rows_affected = executor.execute(&self.sql, &params).await?;
        Ok(QueryResult { rows_affected })
    }
}

impl<T> QueryAs<T>
where
    T: FromPgRow,
{
    pub fn bind(mut self, value: impl IntoSqlParam) -> Self {
        self.params.push(value.into_sql_param());
        self
    }

    pub async fn fetch_one(self, executor: impl DbExecutor) -> Result<T> {
        let params = param_refs(&self.params);
        let row = executor.query_one(&self.sql, &params).await?;
        Ok(T::from_pg_row(&row))
    }

    pub async fn fetch_optional(self, executor: impl DbExecutor) -> Result<Option<T>> {
        let params = param_refs(&self.params);
        let row = executor.query_opt(&self.sql, &params).await?;
        Ok(row.as_ref().map(T::from_pg_row))
    }

    pub async fn fetch_all(self, executor: impl DbExecutor) -> Result<Vec<T>> {
        let params = param_refs(&self.params);
        let rows = executor.query(&self.sql, &params).await?;
        Ok(rows.iter().map(T::from_pg_row).collect())
    }
}

impl<T> QueryScalar<T>
where
    T: FromSqlOwned + Send + Sync + 'static,
{
    pub fn bind(mut self, value: impl IntoSqlParam) -> Self {
        self.params.push(value.into_sql_param());
        self
    }

    pub async fn fetch_one(self, executor: impl DbExecutor) -> Result<T> {
        let params = param_refs(&self.params);
        let row = executor.query_one(&self.sql, &params).await?;
        Ok(row.get(0))
    }

    pub async fn fetch_optional(self, executor: impl DbExecutor) -> Result<Option<T>> {
        let params = param_refs(&self.params);
        let row = executor.query_opt(&self.sql, &params).await?;
        Ok(row.map(|row| row.get(0)))
    }

    pub async fn fetch_all(self, executor: impl DbExecutor) -> Result<Vec<T>> {
        let params = param_refs(&self.params);
        let rows = executor.query(&self.sql, &params).await?;
        Ok(rows.into_iter().map(|row| row.get(0)).collect())
    }
}

fn param_refs(params: &[Box<dyn ToSql + Sync + Send>]) -> Vec<&(dyn ToSql + Sync)> {
    params
        .iter()
        .map(|param| param.as_ref() as &(dyn ToSql + Sync))
        .collect()
}

pub trait DbExecutor {
    async fn execute(self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64>;
    async fn query_one(self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row>;
    async fn query_opt(self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Option<Row>>;
    async fn query(self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>>;
}

impl DbExecutor for &Pool {
    async fn execute(self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        let client = self.get().await?;
        DbExecutor::execute(&client, sql, params).await
    }

    async fn query_one(self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        let client = self.get().await?;
        DbExecutor::query_one(&client, sql, params).await
    }

    async fn query_opt(self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Option<Row>> {
        let client = self.get().await?;
        DbExecutor::query_opt(&client, sql, params).await
    }

    async fn query(self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        let client = self.get().await?;
        DbExecutor::query(&client, sql, params).await
    }
}

impl DbExecutor for &DeadpoolClient {
    async fn execute(self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        Ok(<DeadpoolClient as GenericClient>::execute(self, sql, params).await?)
    }

    async fn query_one(self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        Ok(<DeadpoolClient as GenericClient>::query_one(self, sql, params).await?)
    }

    async fn query_opt(self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Option<Row>> {
        Ok(<DeadpoolClient as GenericClient>::query_opt(self, sql, params).await?)
    }

    async fn query(self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        Ok(<DeadpoolClient as GenericClient>::query(self, sql, params).await?)
    }
}

impl DbExecutor for &Transaction<'_> {
    async fn execute(self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64> {
        Ok(<Transaction<'_> as GenericClient>::execute(self, sql, params).await?)
    }

    async fn query_one(self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        Ok(<Transaction<'_> as GenericClient>::query_one(self, sql, params).await?)
    }

    async fn query_opt(self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Option<Row>> {
        Ok(<Transaction<'_> as GenericClient>::query_opt(self, sql, params).await?)
    }

    async fn query(self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        Ok(<Transaction<'_> as GenericClient>::query(self, sql, params).await?)
    }
}

pub trait IntoSqlParam {
    fn into_sql_param(self) -> Box<dyn ToSql + Sync + Send>;
}

macro_rules! impl_param_owned {
    ($($type:ty),+ $(,)?) => {
        $(
            impl IntoSqlParam for $type {
                fn into_sql_param(self) -> Box<dyn ToSql + Sync + Send> {
                    Box::new(self)
                }
            }
        )+
    };
}

impl_param_owned!(
    bool,
    i16,
    i32,
    i64,
    f32,
    f64,
    String,
    Uuid,
    Value,
    DateTime<Utc>,
    Option<bool>,
    Option<i16>,
    Option<i32>,
    Option<i64>,
    Option<f32>,
    Option<f64>,
    Option<String>,
    Option<Uuid>,
    Option<Value>,
    Option<DateTime<Utc>>,
    Vec<String>,
);

impl IntoSqlParam for &str {
    fn into_sql_param(self) -> Box<dyn ToSql + Sync + Send> {
        Box::new(self.to_owned())
    }
}

impl IntoSqlParam for &String {
    fn into_sql_param(self) -> Box<dyn ToSql + Sync + Send> {
        Box::new(self.clone())
    }
}

impl IntoSqlParam for Option<&str> {
    fn into_sql_param(self) -> Box<dyn ToSql + Sync + Send> {
        Box::new(self.map(str::to_owned))
    }
}

impl IntoSqlParam for Option<&String> {
    fn into_sql_param(self) -> Box<dyn ToSql + Sync + Send> {
        Box::new(self.cloned())
    }
}

impl IntoSqlParam for &Option<String> {
    fn into_sql_param(self) -> Box<dyn ToSql + Sync + Send> {
        Box::new(self.clone())
    }
}

impl IntoSqlParam for &Option<i32> {
    fn into_sql_param(self) -> Box<dyn ToSql + Sync + Send> {
        Box::new(*self)
    }
}

impl IntoSqlParam for &Option<Value> {
    fn into_sql_param(self) -> Box<dyn ToSql + Sync + Send> {
        Box::new(self.clone())
    }
}

impl IntoSqlParam for &Uuid {
    fn into_sql_param(self) -> Box<dyn ToSql + Sync + Send> {
        Box::new(*self)
    }
}

impl IntoSqlParam for &Value {
    fn into_sql_param(self) -> Box<dyn ToSql + Sync + Send> {
        Box::new(self.clone())
    }
}

impl IntoSqlParam for &[String] {
    fn into_sql_param(self) -> Box<dyn ToSql + Sync + Send> {
        Box::new(self.to_vec())
    }
}

impl IntoSqlParam for &Vec<String> {
    fn into_sql_param(self) -> Box<dyn ToSql + Sync + Send> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_mode_honors_postgres_sslmode() {
        assert_eq!(
            tls_mode("postgresql://localhost/spotrak"),
            TlsMode::Unverified
        );
        assert_eq!(
            tls_mode("postgresql://localhost/spotrak?sslmode=disable"),
            TlsMode::Disable
        );
        assert_eq!(
            tls_mode("postgresql://localhost/spotrak?sslmode=verify-ca"),
            TlsMode::VerifyFull
        );
        assert_eq!(
            tls_mode("postgresql://localhost/spotrak?sslmode=verify-full"),
            TlsMode::VerifyFull
        );
    }

    #[test]
    fn migration_without_transaction_requires_leading_marker() {
        assert!(migration_without_transaction(
            "\n-- no-transaction\nCREATE INDEX CONCURRENTLY idx ON events (id);"
        ));
        assert!(migration_without_transaction("-- NO-TRANSACTION\nVACUUM;"));
        assert!(!migration_without_transaction(
            "CREATE TABLE events (id bigint);\n-- no-transaction"
        ));
        assert!(!migration_without_transaction(
            "-- ordinary comment\nCREATE TABLE events (id bigint);"
        ));
    }
}
