mod account_transaction_processor;
mod processor;

use std::collections::HashMap;

use thiserror::Error;

use crate::model::{Amount, ClientId, TransactionId};
use crate::transaction_processor::{
    Transaction as RawTransaction, TransactionKind as RawTransactionKind,
};

/// The snapshot of an account.
/// An account consists of a series of chronologically ordered transactions
/// and the account's state is determined by these ordered transactions.
/// To capture the account's state, replaying all these transactions is time
/// consuming and a snapshot is helpful to keep track of certain key attributes
/// of an account.
#[derive(Debug, PartialEq)]
struct AccountSnapshot {
    available: Amount,
    held: Amount,
}

#[derive(Debug, PartialEq)]
enum AccountStatus {
    /// The account is active, and is open to transactions.
    Active,

    /// The account is locked and is closed to transactions.
    Locked,
}

/// An account structure used to process transactions.
#[derive(Debug, PartialEq)]
pub(crate) struct Account {
    client_id: ClientId,
    status: AccountStatus,
    account_snapshot: AccountSnapshot,
    deposits: HashMap<TransactionId, Deposit>,
    withdrawals: HashMap<TransactionId, Withdrawal>,
}

/// A transaction that is ready to be processed for an account.
#[derive(Debug, PartialEq)]
pub(crate) struct Transaction {
    transaction_id: TransactionId,
    kind: TransactionKind,
}

/// The kind of transaction to be processed for an account.
#[derive(Debug, PartialEq)]
pub(crate) enum TransactionKind {
    DepositTransaction { amount: Amount },
    WithdrawalTransaction { amount: Amount },
    DisputeTransaction,
    ResolveTransaction,
    ChargeBackTransaction,
}

impl From<RawTransaction> for Transaction {
    fn from(raw: RawTransaction) -> Self {
        Self {
            transaction_id: raw.transaction_id,
            kind: match raw.kind {
                RawTransactionKind::DepositTransaction { amount } => {
                    TransactionKind::DepositTransaction { amount }
                }
                RawTransactionKind::WithdrawalTransaction { amount } => {
                    TransactionKind::WithdrawalTransaction { amount }
                }
                RawTransactionKind::DisputeTransaction => TransactionKind::DisputeTransaction,
                RawTransactionKind::ResolveTransaction => TransactionKind::ResolveTransaction,
                RawTransactionKind::ChargeBackTransaction => TransactionKind::ChargeBackTransaction,
            },
        }
    }
}

#[derive(Debug, PartialEq)]
enum DepositStatus {
    /// This is the initial state of an accepted deposit.
    Accepted,

    /// An accepted deposit can be disputed.
    /// Once a dispute transaction with the same [`TransactionId`] is received,
    /// the deposit is put on hold.
    /// An on-hold deposit will be either resolved or charged back, depending
    /// on the subsequent transaction that concludes it.
    Held,

    /// A disputed deposit can be resolved.
    /// Once resolved, the funds associated with the deposit will be available.
    Resolved,

    /// A disputed deposit can be charged back.
    /// Once charged back, the deposit will be reversed.
    ChargedBack,
}

#[derive(Debug, PartialEq)]
struct Deposit {
    amount: Amount,
    status: DepositStatus,
}

#[derive(Debug, PartialEq)]
enum WithdrawalStatus {
    /// This is the initial state of an accepted withdrawal.
    Accepted,

    /// The account did not have sufficient fund for the withdrawal.
    /// This transaction does not have an effect on the funds of the account.
    Rejected,

    /// An accepted withdrawal can be disputed.
    /// Once a dispute transaction with the same [`TransactionId`] is received,
    /// the withdrawal is put on hold.
    /// An on-hold withdrawal will be either resolved or charged back,
    /// depending on the subsequent transaction that concludes it.
    Held,

    /// A disputed withdrawal can be resolved.
    /// Once resolved, the funds associated with the withdrawal will be
    /// effective on the available amount.
    Resolved,

    /// A disputed withdrawal can be charged back.
    /// Once charged back, the withdrawal will be reversed.
    ChargedBack,
}

#[derive(Debug, PartialEq)]
struct Withdrawal {
    amount: Amount,
    status: WithdrawalStatus,
}

/// A trait that specify a storage of an account.
pub(crate) trait AccountStore {
    /// Get the account from the [`AccountStore`]
    fn get(&self, client_id: ClientId) -> Result<Option<Account>, AccountStoreError>;

    /// Create an account in the [`AccountStore`]
    fn create(&self, client_id: ClientId) -> Result<Account, AccountStoreError>;

    /// List all accounts
    fn list(&self) -> Result<Vec<Account>, AccountStoreError>;
}

#[derive(Debug, Error)]
pub(crate) enum AccountStoreError {}

#[cfg(test)]
impl AccountSnapshot {
    fn new(available: i32, held: u32) -> Self {
        use ordered_float::OrderedFloat;
        AccountSnapshot {
            available: OrderedFloat(available as f32),
            held: OrderedFloat(held as f32),
        }
    }
    fn empty() -> Self {
        Self::new(0, 0)
    }
}
