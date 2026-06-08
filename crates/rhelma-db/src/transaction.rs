use crate::{DbError, DbResult};
use futures::future::BoxFuture;
use sqlx::{Postgres, Transaction};
use std::time::Duration;
use tokio::time::sleep;
use tracing::instrument;

#[derive(Clone, Copy, Debug)]
pub struct TransactionRetryPolicy {
    /// Field `max_retries`.
    pub max_retries: u32,
    /// Field `base_backoff`.
    pub base_backoff: Duration,
    /// Field `max_backoff`.
    pub max_backoff: Duration,
}

impl Default for TransactionRetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_backoff: Duration::from_millis(25),
            max_backoff: Duration::from_millis(250),
        }
    }
}

//fn is_retryable_sqlx(err: &sqlx::Error) -> bool {
//   match err {
//      sqlx::Error::Database(db) => {
//           match db.code().as_deref() {
//               Some("40001") => true,  // serialization_failure
//               Some("40P01") => true,  // deadlock_detected
//              _ => false,
//           }
//       }
//        sqlx::Error::PoolTimedOut => true,
//        _ => false,
//    }
//}

/// ساده (بدون retry)
#[instrument(skip(f, pool))]
pub async fn with_transaction<T, F>(pool: &sqlx::PgPool, f: F) -> DbResult<T>
where
    F: for<'a> FnOnce(&'a mut Transaction<'_, Postgres>) -> BoxFuture<'a, DbResult<T>>,
{
    let mut tx = pool.begin().await.map_err(DbError::from_sqlx)?;

    match f(&mut tx).await {
        Ok(result) => {
            tx.commit().await.map_err(DbError::from_sqlx)?;
            Ok(result)
        }
        Err(e) => {
            tx.rollback().await.map_err(DbError::from_sqlx)?;
            Err(e)
        }
    }
}

/// retryable (برای serialization/deadlock)
#[instrument(skip(f, pool))]
pub async fn with_transaction_retry<T, F>(
    pool: &sqlx::PgPool,
    mut f: F,
    policy: TransactionRetryPolicy,
) -> DbResult<T>
where
    F: for<'a> FnMut(&'a mut Transaction<'_, Postgres>) -> BoxFuture<'a, DbResult<T>>,
{
    let mut attempt: u32 = 0;

    loop {
        let mut tx = pool.begin().await.map_err(DbError::from_sqlx)?;

        let res = f(&mut tx).await;

        match res {
            Ok(v) => {
                tx.commit().await.map_err(DbError::from_sqlx)?;
                return Ok(v);
            }
            Err(e) => {
                // اگر خطای داخل closure از نوع DbError است، ممکنه ریتری‌بل نباشه.
                // اما اگر rollback/commit به sqlx خطا خورد، اون رو هم بررسی می‌کنیم.
                let _ = tx.rollback().await;

                // اگر e خودش DbError بود، retry تصمیم را فقط با rollback sqlx نمی‌گیریم.
                // برای retry واقعی باید سرویس/ریپو DbError::Connection{code} را set کند
                // یا اینجا از classification استفاده کنیم.
                // فعلاً retry فقط روی sqlx begin/commit/rollback و pool-timeout منطقی است.

                attempt += 1;
                if attempt > policy.max_retries {
                    return Err(e);
                }

                // backoff ساده
                let mut backoff = policy.base_backoff * attempt;
                if backoff > policy.max_backoff {
                    backoff = policy.max_backoff;
                }
                sleep(backoff).await;

                continue;
            }
        }
    }
}
