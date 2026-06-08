//! Deposit hold scaffolding (Value Ledger Federation integration).
//!
//! For Phase 45, this is intentionally a stub. Wire it when VLF endpoints are finalized.

#[derive(Debug, Clone)]
pub struct DepositHold {
    /// Field `subject_id`.
    pub subject_id: String,
    /// Field `amount`.
    pub amount: i64,
    /// Field `hold_ref`.
    pub hold_ref: String,
}
