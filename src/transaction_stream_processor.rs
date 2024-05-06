pub mod async_csv_stream_processor;
pub mod csv_stream_processor;
mod transaction_record_converter;

use std::{io::Read, num::ParseFloatError};

use async_trait::async_trait;

use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

use crate::{
    model::{ClientId, TransactionId},
    transaction_processor::TransactionProcessorError,
};

#[async_trait]
pub trait TransactionStreamProcessor {
    async fn process(&self, r: impl Read + Send) -> Result<(), TransactionStreamProcessError>;
}

#[derive(Debug, Error)]
pub enum TransactionStreamProcessError {
    #[error("Error occurred during parsing the input data: {0}")]
    ParsingError(String),
    #[error("Error occurred during processing the `TransactionRecord` {0:?} due to {1}")]
    ProcessError(TransactionRecord, String),
    #[error("Failed to shutdown the processor: {0}")]
    FailedToShutdown(String),
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct TransactionRecord {
    #[serde(rename = "type")]
    pub txn_type: TransactionRecordType,
    #[serde(rename = "client")]
    pub client_id: ClientId,
    #[serde(rename = "tx")]
    pub transaction_id: TransactionId,
    #[serde(rename = "amount")]
    pub optional_amount: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
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

impl From<TransactionProcessorError> for TransactionStreamProcessError {
    fn from(err: TransactionProcessorError) -> Self {
        match err {
            TransactionProcessorError::AccountLocked => todo!(),
            TransactionProcessorError::InvalidTransaction(_) => todo!(),
            TransactionProcessorError::InternalError(_) => todo!(),
        }
    }
}

impl From<ParseFloatError> for TransactionStreamProcessError {
    fn from(err: ParseFloatError) -> Self {
        Self::ParsingError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use dashmap::DashMap;
    use rstest::rstest;
    use rstest_reuse::{apply, template};

    use crate::account::AccountStatus::Active;
    use crate::account::DepositStatus::Accepted;
    use crate::account::{Account, AccountSnapshot, Deposit, SimpleAccountTransactor, Withdrawal};
    use crate::transaction_stream_processor::async_csv_stream_processor::AsyncCsvStreamProcessor;
    use crate::transaction_stream_processor::csv_stream_processor::CsvStreamProcessor;
    use crate::transaction_stream_processor::TransactionStreamProcessor;

    use crate::model::{
        Amount4DecimalBased, ClientId, Transaction, TransactionId, TransactionKind,
    };
    use crate::transaction_processor::{RecordSink, SimpleTransactionProcessor};

    #[template]
    #[rstest]
    #[case("
    type,    client, tx, amount
    deposit,      1,  2,    3.0",
            vec![deposit(1, 2, 30_000)])]
    #[case("
    type,       client, tx, amount
    withdrawal,      4,  5,    6.0",
            vec![withdrawal(4, 5, 60_000)])]
    #[case("
    type,    client, tx, amount
    dispute,      7,  8,       ",
            vec![dispute(7, 8)])]
    #[case("
    type,    client, tx, amount
    resolve,      9, 10,       ",
            vec![resolve(9, 10)])]
    #[case("
    type,       client, tx, amount
    chargeback,     11, 12,       ",
            vec![chargeback(11, 12)])]
    #[case("
    type,       client,  tx, amount
    deposit,         1,  2,     3.0
    withdrawal,      1,  5,     6.0
    dispute,         7,  8,
    resolve,         9, 10,
    chargeback,     11, 12,",
            vec![deposit(1, 2, 30_000),
            withdrawal(1, 5, 60_000),
            dispute(7, 8),
            resolve(9, 10),
            chargeback(11, 12)])]
    fn valid_csv_cases(#[case] input: &str, #[case] expected: Vec<Transaction>) {}

    #[apply(valid_csv_cases)]
    #[tokio::test]
    async fn csv_parsing_works_for_async_stream_processor(
        #[case] input: &str,
        #[case] expected: Vec<Transaction>,
    ) {
        let records = Arc::new(Mutex::new(Vec::new()));
        let record_sink = RecordSink {
            records: records.clone(),
        };
        let senders_and_handles = DashMap::new();

        let processor = AsyncCsvStreamProcessor::new(Arc::new(record_sink), senders_and_handles);
        processor.process(input.as_bytes()).await.unwrap();
        processor.shutdown().await.unwrap();
        assert_eq!(*records.lock().unwrap(), expected);
    }

    #[apply(valid_csv_cases)]
    #[tokio::test]
    async fn csv_parsing_works_for_simple_stream_processor(
        #[case] input: &str,
        #[case] expected: Vec<Transaction>,
    ) {
        let records = Arc::new(Mutex::new(Vec::new()));
        let record_sink = RecordSink {
            records: records.clone(),
        };
        let processor = CsvStreamProcessor::new(Box::new(record_sink));
        processor.process(input.as_bytes()).await.unwrap();
        assert_eq!(*records.lock().unwrap(), expected);
    }

    #[tokio::test]
    async fn e2_account_storage_with_small_input_using_async_processor() {
        let accounts = Arc::new(DashMap::new());
        let account_transaction_processor = SimpleAccountTransactor::new();
        let transaction_processor = SimpleTransactionProcessor::new(
            accounts.clone(),
            Box::new(account_transaction_processor),
        );
        let senders_and_handles = DashMap::new();

        let processor =
            AsyncCsvStreamProcessor::new(Arc::new(transaction_processor), senders_and_handles);

        let input = "
       type, client, tx, amount
    deposit,      1, 10,    4.0
    deposit,      1, 20,    5.0
    deposit,      2, 30,    6.0";

        let mut client_1_deposits = HashMap::new();
        client_1_deposits.insert(10, accepted_deposit(40_000));
        client_1_deposits.insert(20, accepted_deposit(50_000));

        let mut client_2_deposits = HashMap::new();
        client_2_deposits.insert(30, accepted_deposit(60_000));

        let mut expected_accounts = HashMap::new();
        expected_accounts.insert(
            1,
            active_account(1, snapshot(90_000, 0), client_1_deposits, HashMap::new()),
        );
        expected_accounts.insert(
            2,
            active_account(2, snapshot(60_000, 0), client_2_deposits, HashMap::new()),
        );

        processor.process(input.as_bytes()).await.unwrap();
        processor.shutdown().await.unwrap();
        assert_eq!(accounts.len(), expected_accounts.len());
        accounts.iter().for_each(|entry| {
            let key = entry.key();
            let value = entry.value();
            assert_eq!(value, expected_accounts.get(key).unwrap());
        });
    }

    fn deposit(client_id: ClientId, transaction_id: TransactionId, amount: i64) -> Transaction {
        Transaction {
            client_id,
            transaction_id,
            kind: TransactionKind::Deposit {
                amount: Amount4DecimalBased(amount),
            },
        }
    }

    fn withdrawal(client_id: ClientId, transaction_id: TransactionId, amount: i64) -> Transaction {
        Transaction {
            client_id,
            transaction_id,
            kind: TransactionKind::Withdrawal {
                amount: Amount4DecimalBased(amount),
            },
        }
    }

    fn dispute(client_id: ClientId, transaction_id: TransactionId) -> Transaction {
        Transaction {
            client_id,
            transaction_id,
            kind: TransactionKind::Dispute,
        }
    }

    fn resolve(client_id: ClientId, transaction_id: TransactionId) -> Transaction {
        Transaction {
            client_id,
            transaction_id,
            kind: TransactionKind::Resolve,
        }
    }

    fn chargeback(client_id: ClientId, transaction_id: TransactionId) -> Transaction {
        Transaction {
            client_id,
            transaction_id,
            kind: TransactionKind::ChargeBack,
        }
    }

    fn snapshot(available: i64, held: i64) -> AccountSnapshot {
        AccountSnapshot::new(available, held)
    }

    fn active_account(
        client_id: ClientId,
        account_snapshot: AccountSnapshot,
        deposits: HashMap<TransactionId, Deposit>,
        withdrawals: HashMap<TransactionId, Withdrawal>,
    ) -> Account {
        Account::new(client_id, Active, account_snapshot, deposits, withdrawals)
    }

    fn accepted_deposit(amount: i64) -> Deposit {
        Deposit {
            amount: Amount4DecimalBased(amount),
            status: Accepted,
        }
    }
}
