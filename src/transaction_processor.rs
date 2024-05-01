use crate::model::{Amount, ClientId, TransactionId};

/// The transaction structure accepted by this application.
pub struct Transaction {
    client_id: ClientId,
    pub transaction_id: TransactionId,
    pub kind: TransactionKind,
}

/// The kinds of transactions.
pub enum TransactionKind {
    DepositTransaction { amount: Amount },
    WithdrawalTransaction { amount: Amount },
    DisputeTransaction,
    ResolveTransaction,
    ChargeBackTransaction,
}

/// The transction processor.
/// It takes in a transaction and processes it based on previously seen
/// transactions. The transaction may be rejected if there is an error occurred
/// during the process of it.
trait TransactionProcessor {
    fn process(&self, transaction: Transaction) -> Result<(), TransactionProcessorError>;
}

pub(super) enum TransactionProcessorError {
    InvalidTransaction(Transaction),
}
