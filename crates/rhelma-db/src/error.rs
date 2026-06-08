use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    /// Generic DB connectivity/query error (sanitized).
    #[error("database error")]
    /// Variant `Connection`.
    Connection { code: Option<String> },

    #[error("migration error")]
    /// Variant `Migration`.
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("transaction error: {0}")]
    /// Variant `Transaction`.
    Transaction(String),

    /// Constraint violation (sanitized; no raw message).
    #[error("constraint violation")]
    /// Variant `Constraint`.
    Constraint {
        code: Option<String>,
        constraint: Option<String>,
    },

    #[error("not found: {0}")]
    /// Variant `NotFound`.
    NotFound(String),

    /// Residency policy violation (block DB access).
    #[error("residency violation")]
    /// Variant `ResidencyViolation`.
    ResidencyViolation {
        tenant: Option<String>,
        requested_region: Option<String>,
        db_region: Option<String>,
    },
}

pub type DbResult<T> = Result<T, DbError>;

impl DbError {
    pub fn from_sqlx(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => DbError::NotFound("row not found".into()),

            sqlx::Error::Database(db_err) => {
                let code = db_err.code().map(|c| c.to_string());

                // Postgres constraint class is usually 23xxx
                let is_constraint = code
                    .as_deref()
                    .map(|c| c.starts_with("23"))
                    .unwrap_or(false);

                if is_constraint {
                    let constraint = db_err.constraint().map(|s| s.to_string());
                    DbError::Constraint { code, constraint }
                } else {
                    DbError::Connection { code }
                }
            }

            // Any other sqlx error => sanitized connection error
            other => {
                // Try to keep a tiny bit of signal without leaking details
                let code = match &other {
                    sqlx::Error::Protocol(_) => Some("protocol".into()),
                    sqlx::Error::Io(_) => Some("io".into()),
                    sqlx::Error::Tls(_) => Some("tls".into()),
                    sqlx::Error::PoolTimedOut => Some("pool_timeout".into()),
                    _ => None,
                };
                DbError::Connection { code }
            }
        }
    }
}

impl From<DbError> for rhelma_core::RhelmaError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::NotFound(m) => rhelma_core::RhelmaError::NotFound(m),
            DbError::Constraint { .. } => {
                rhelma_core::RhelmaError::Conflict("constraint violation".into())
            }
            DbError::ResidencyViolation { .. } => {
                rhelma_core::RhelmaError::SecurityPolicy("residency violation".into())
            }
            DbError::Migration(_) => rhelma_core::RhelmaError::Database("migration error".into()),
            DbError::Transaction(m) => rhelma_core::RhelmaError::Database(m),
            DbError::Connection { .. } => {
                rhelma_core::RhelmaError::Database("database error".into())
            }
        }
    }
}
