
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct PersistentLedgerStore {
    db: PgPool,
    cache: Arc<RwLock<HashMap<String, Balance>>>,
}

impl PersistentLedgerStore {
    pub async fn record_transaction(&self, tx: Transaction) -> Result<()> {
        // Write to DB (source of truth)
        sqlx::query!(
            r#"
            INSERT INTO value_ledger_transactions 
            (tx_id, subject_id, delta, reason, created_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            tx.id, tx.subject_id, tx.delta, tx.reason, tx.created_at
        )
        .execute(&self.db)
        .await?;
        
        // Update cache
        let mut cache = self.cache.write().await;
        cache.entry(tx.subject_id.clone())
            .and_modify(|b| b.balance += tx.delta)
            .or_insert(Balance { 
                balance: tx.delta,
                updated_at: tx.created_at 
            });
        
        Ok(())
    }
    
    pub async fn get_history(&self, subject_id: &str) -> Vec<Transaction> {
        sqlx::query_as!(
            Transaction,
            "SELECT * FROM value_ledger_transactions 
             WHERE subject_id = $1 
             ORDER BY created_at DESC",
            subject_id
        )
        .fetch_all(&self.db)
        .await
        .unwrap_or_default()
    }
}
