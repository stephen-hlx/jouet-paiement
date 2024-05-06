use serde::Serialize;

mod account_summary;
mod amount;

pub type ClientId = u16;
pub type TransactionId = u32;
pub type Amount = Amount4DecimalBased;

/// The transaction structure accepted by this application.
#[derive(Debug, PartialEq, Clone)]
pub struct Transaction {
    pub client_id: ClientId,
    pub transaction_id: TransactionId,
    pub kind: TransactionKind,
}

/// The kinds of transactions.
#[derive(Debug, PartialEq, Clone)]
pub enum TransactionKind {
    Deposit { amount: Amount },
    Withdrawal { amount: Amount },
    Dispute,
    Resolve,
    ChargeBack,
}

/// TODO: Use proper serde to avoid having `String`s as the type of fields.
#[derive(Debug, Serialize)]
pub struct AccountSummary {
    #[serde(rename = "client")]
    client_id: ClientId,
    #[serde(rename = "available")]
    available: String,
    #[serde(rename = "held")]
    held: String,
    #[serde(rename = "total")]
    total: String,
    #[serde(rename = "locked")]
    locked: bool,
}

/// The amount is stored as an i64 to simplify the handling of precision.
/// The downside of doing so is that it could only hold up to the amount of
/// `i64::MAX / 10_000`.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Amount4DecimalBased(pub i64);
