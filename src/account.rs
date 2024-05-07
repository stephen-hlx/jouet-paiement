pub(crate) mod account_transactor;
pub use account_transactor::SimpleAccountTransactor;
mod transactors;

use std::collections::HashMap;

use thiserror::Error;

use crate::model::{Amount, Amount4DecimalBased, ClientId, TransactionId};

/// The snapshot of an account.
/// An account consists of a series of chronologically ordered transactions
/// and the account's state is determined by these ordered transactions.
/// To capture the account's state, replaying all these transactions is time
/// consuming and a snapshot is helpful to keep track of certain key attributes
/// of an account.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct AccountSnapshot {
    pub(crate) available: Amount,
    pub(crate) held: Amount,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum AccountStatus {
    /// The account is active, and is open to transactions.
    Active,

    /// The account is locked and is closed to transactions.
    Locked,
}

/// An account structure used to process transactions.
#[derive(Debug, PartialEq, Clone)]
pub struct Account {
    pub(crate) client_id: ClientId,
    pub(crate) status: AccountStatus,
    pub(crate) account_snapshot: AccountSnapshot,
    deposits: HashMap<TransactionId, Deposit>,
    withdrawals: HashMap<TransactionId, Withdrawal>,
}

impl Account {
    pub(crate) fn active(client_id: ClientId) -> Self {
        Account {
            client_id,
            status: AccountStatus::Active,
            account_snapshot: AccountSnapshot::empty(),
            deposits: HashMap::new(),
            withdrawals: HashMap::new(),
        }
    }

    #[cfg(test)]
    pub fn new(
        client_id: ClientId,
        status: AccountStatus,
        account_snapshot: AccountSnapshot,
        deposits: HashMap<TransactionId, Deposit>,
        withdrawals: HashMap<TransactionId, Withdrawal>,
    ) -> Self {
        Self {
            client_id,
            status,
            account_snapshot,
            deposits,
            withdrawals,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DepositStatus {
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

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Deposit {
    pub amount: Amount,
    pub status: DepositStatus,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum WithdrawalStatus {
    /// This is the initial state of an accepted withdrawal.
    Accepted,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Withdrawal {
    amount: Amount,
    status: WithdrawalStatus,
}

#[derive(Debug, Error)]
pub(crate) enum AccountStoreError {}

impl AccountSnapshot {
    pub fn new(available: i64, held: i64) -> Self {
        AccountSnapshot {
            available: Amount4DecimalBased(available),
            held: Amount4DecimalBased(held),
        }
    }
    pub(crate) fn empty() -> Self {
        Self::new(0, 0)
    }
}
