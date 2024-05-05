use ordered_float::OrderedFloat;

pub type ClientId = u16;
pub type TransactionId = u32;
pub type Amount = OrderedFloat<f32>;

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
