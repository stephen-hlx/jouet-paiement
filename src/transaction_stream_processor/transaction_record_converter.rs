use crate::model::{Amount, Transaction, TransactionKind};

use super::{TransactionRecord, TransactionRecordType, TransactionStreamProcessError};

// TODO:
// This whole function could have been avoided if the deserialsation can be
// implemented directly on top of `Transaction` instead of going through
// `TransactionRecord`.
pub(super) fn to_transaction(
    record: TransactionRecord,
) -> Result<Transaction, TransactionStreamProcessError> {
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
                    Some(amount) => Amount::from_str(&amount)?,
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
                    Some(amount) => Amount::from_str(&amount)?,
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
    use rstest::rstest;

    use crate::transaction_stream_processor::transaction_record_converter::to_transaction;

    use crate::model::{Amount, ClientId, Transaction, TransactionId, TransactionKind};

    use super::{TransactionRecord, TransactionRecordType};

    const CLIENT_ID: ClientId = 1234;
    const TRANSACTION_ID: TransactionId = 5678;
    const AMOUNT: &'static str = "0.9";

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

    fn deposit_transaction(amount: &str) -> Transaction {
        transaction(TransactionKind::Deposit {
            amount: Amount::from_str(&amount).unwrap(),
        })
    }

    fn withdrawal_transaction(amount: &str) -> Transaction {
        transaction(TransactionKind::Withdrawal {
            amount: Amount::from_str(&amount).unwrap(),
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

    fn deposit_record(optional_amount: Option<&str>) -> TransactionRecord {
        transaction_record(TransactionRecordType::Deposit, optional_amount)
    }

    fn withdrawal_record(optional_amount: Option<&str>) -> TransactionRecord {
        transaction_record(TransactionRecordType::Withdrawal, optional_amount)
    }

    fn dispute_record(optional_amount: Option<&str>) -> TransactionRecord {
        transaction_record(TransactionRecordType::Dispute, optional_amount)
    }

    fn resolve_record(optional_amount: Option<&str>) -> TransactionRecord {
        transaction_record(TransactionRecordType::Resolve, optional_amount)
    }

    fn chargeback_record(optional_amount: Option<&str>) -> TransactionRecord {
        transaction_record(TransactionRecordType::Chargeback, optional_amount)
    }

    fn transaction_record(
        txn_type: TransactionRecordType,
        optional_amount: Option<&str>,
    ) -> TransactionRecord {
        TransactionRecord {
            txn_type,
            client_id: CLIENT_ID,
            transaction_id: TRANSACTION_ID,
            optional_amount: optional_amount.map(|s| s.to_string()),
        }
    }
}
