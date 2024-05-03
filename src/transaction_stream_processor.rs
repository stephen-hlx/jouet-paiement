use std::io;

use ordered_float::OrderedFloat;
use serde::Deserialize;
use thiserror::Error;

use crate::{
    model::{ClientId, TransactionId},
    transaction_processor::{Transaction, TransactionKind},
};

trait TransactionStreamProcessor {
    fn process<R: io::Read>(&mut self, r: R) -> Result<(), TransactionStreamProcessError>;
}

#[derive(Debug, Error)]
pub enum TransactionStreamProcessError {
    #[error("Error occurred during parsing the input data: {0}")]
    ParsingError(String),
    #[error("Error occurred during processing the `TransactionRecord` {0:?} due to {1}")]
    ProcessError(TransactionRecord, String),
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct TransactionRecord {
    #[serde(rename = "type")]
    pub txn_type: TransactionRecordType,
    #[serde(rename = "client")]
    pub client_id: ClientId,
    #[serde(rename = "tx")]
    pub transaction_id: TransactionId,
    #[serde(rename = "amount")]
    pub optional_amount: Option<f32>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub enum TransactionRecordType {
    #[serde(rename = "deposit")]
    Deposit,
    #[serde(rename = "withdrawal")]
    Withdrawal,
    #[serde(rename = "dispute")]
    Dispute,
    #[serde(rename = "resolve")]
    Resolve,
    #[serde(rename = "chargeback")]
    Chargeback,
}

mod csv_stream_processor;

trait TransactionConsumer {
    fn consume(&mut self, transaction: Transaction) -> Result<(), TransactionConsumerError>;
}

#[derive(Debug, Error)]
#[error("Failed to consumer the `TransactionRecord` {0:?} due to {1}")]
struct TransactionConsumerError(TransactionRecord, String);

impl From<TransactionConsumerError> for TransactionStreamProcessError {
    fn from(value: TransactionConsumerError) -> Self {
        Self::ProcessError(value.0, value.1)
    }
}

// TODO:
// This whole function could have been avoided if the deserialsation can be
// implemented directly on top of `Transaction` instead of going through
// `TransactionRecord`.
fn to_transaction(record: TransactionRecord) -> Result<Transaction, TransactionStreamProcessError> {
    let TransactionRecord {
        txn_type,
        client_id,
        transaction_id,
        optional_amount,
    } = record;
    let transaction = match txn_type {
        TransactionRecordType::Deposit => Transaction {
            client_id,
            transaction_id,
            kind: TransactionKind::Deposit {
                amount: match optional_amount {
                    Some(amount) => OrderedFloat(amount),
                    None => {
                        return Err(TransactionStreamProcessError::ParsingError(
                            "Amount not found for deposit.".to_string(),
                        ))
                    }
                },
            },
        },
        TransactionRecordType::Withdrawal => Transaction {
            client_id,
            transaction_id,
            kind: TransactionKind::Withdrawal {
                amount: match optional_amount {
                    Some(amount) => OrderedFloat(amount),
                    None => {
                        return Err(TransactionStreamProcessError::ParsingError(
                            "Amount not found for withdrawal.".to_string(),
                        ))
                    }
                },
            },
        },
        TransactionRecordType::Dispute => Transaction {
            client_id,
            transaction_id,
            kind: TransactionKind::Dispute,
        },
        TransactionRecordType::Resolve => Transaction {
            client_id,
            transaction_id,
            kind: TransactionKind::Resolve,
        },
        TransactionRecordType::Chargeback => Transaction {
            client_id,
            transaction_id,
            kind: TransactionKind::ChargeBack,
        },
    };
    Ok(transaction)
}

#[cfg(test)]
mod tests {
    use ordered_float::OrderedFloat;
    use rstest::rstest;

    use crate::{
        model::{ClientId, TransactionId},
        transaction_processor::{Transaction, TransactionKind},
    };

    use super::{TransactionRecord, TransactionRecordType};
    use crate::transaction_stream_processor::to_transaction;

    const CLIENT_ID: ClientId = 1234;
    const TRANSACTION_ID: TransactionId = 5678;
    const AMOUNT: f32 = 0.9;

    #[rstest]
    #[case(deposit_record(Some(AMOUNT)), deposit_transaction(AMOUNT))]
    #[case(withdrawal_record(Some(AMOUNT)), withdrawal_transaction(AMOUNT))]
    #[case(dispute_record(None), dispute_transaction())]
    #[case(resolve_record(None), resolve_transaction())]
    #[case(chargeback_record(None), chargeback_transaction())]
    fn conversion_from_transaction_record_to_transaction_works(
        #[case] transaction_record: TransactionRecord,
        #[case] expected: Transaction,
    ) {
        assert_eq!(to_transaction(transaction_record).unwrap(), expected);
    }

    fn deposit_transaction(amount: f32) -> Transaction {
        transaction(TransactionKind::Deposit {
            amount: OrderedFloat(amount),
        })
    }

    fn withdrawal_transaction(amount: f32) -> Transaction {
        transaction(TransactionKind::Withdrawal {
            amount: OrderedFloat(amount),
        })
    }

    fn dispute_transaction() -> Transaction {
        transaction(TransactionKind::Dispute {})
    }

    fn resolve_transaction() -> Transaction {
        transaction(TransactionKind::Resolve {})
    }

    fn chargeback_transaction() -> Transaction {
        transaction(TransactionKind::ChargeBack {})
    }

    fn transaction(kind: TransactionKind) -> Transaction {
        Transaction {
            client_id: CLIENT_ID,
            transaction_id: TRANSACTION_ID,
            kind,
        }
    }

    fn deposit_record(optional_amount: Option<f32>) -> TransactionRecord {
        transaction_record(TransactionRecordType::Deposit, optional_amount)
    }

    fn withdrawal_record(optional_amount: Option<f32>) -> TransactionRecord {
        transaction_record(TransactionRecordType::Withdrawal, optional_amount)
    }

    fn dispute_record(optional_amount: Option<f32>) -> TransactionRecord {
        transaction_record(TransactionRecordType::Dispute, optional_amount)
    }

    fn resolve_record(optional_amount: Option<f32>) -> TransactionRecord {
        transaction_record(TransactionRecordType::Resolve, optional_amount)
    }

    fn chargeback_record(optional_amount: Option<f32>) -> TransactionRecord {
        transaction_record(TransactionRecordType::Chargeback, optional_amount)
    }

    fn transaction_record(
        txn_type: TransactionRecordType,
        optional_amount: Option<f32>,
    ) -> TransactionRecord {
        TransactionRecord {
            txn_type,
            client_id: CLIENT_ID,
            transaction_id: TRANSACTION_ID,
            optional_amount,
        }
    }
}
