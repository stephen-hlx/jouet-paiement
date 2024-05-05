pub mod async_csv_stream_processor;
pub mod csv_stream_processor;
mod transaction_record_converter;

use std::io::Read;

use async_trait::async_trait;

use serde::Deserialize;
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

impl From<TransactionProcessorError> for TransactionStreamProcessError {
    fn from(err: TransactionProcessorError) -> Self {
        match err {
            TransactionProcessorError::AccountLocked => todo!(),
            TransactionProcessorError::InvalidTransaction(_) => todo!(),
            TransactionProcessorError::InternalError(_) => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use dashmap::DashMap;
    use ordered_float::OrderedFloat;
    use rstest::rstest;
    use rstest_reuse::{apply, template};

    use crate::transaction_stream_processor::async_csv_stream_processor::AsyncCsvStreamProcessor;
    use crate::transaction_stream_processor::csv_stream_processor::CsvStreamProcessor;
    use crate::transaction_stream_processor::TransactionStreamProcessor;

    use crate::transaction_processor::RecordSink;
    use crate::{
        model::{ClientId, TransactionId},
        transaction_processor::{Transaction, TransactionKind},
    };

    #[template]
    #[rstest]
    #[case("
    type,    client, tx, amount
    deposit,      1,  2,    3.0",
            vec![deposit(1, 2, 3.0)])]
    #[case("
    type,       client, tx, amount
    withdrawal,      4,  5,    6.0",
            vec![withdrawal(4, 5, 6.0)])]
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
    type, client, tx, amount
    deposit, 1, 2, 3.0
    withdrawal, 4, 5, 6.0
    dispute, 7, 8,
    resolve, 9, 10,
    chargeback, 11, 12,",
            vec![deposit(1, 2, 3.0),
            withdrawal(4, 5, 6.0),
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

    fn deposit(client_id: ClientId, transaction_id: TransactionId, amount: f32) -> Transaction {
        Transaction {
            client_id,
            transaction_id,
            kind: TransactionKind::Deposit {
                amount: OrderedFloat(amount),
            },
        }
    }

    fn withdrawal(client_id: ClientId, transaction_id: TransactionId, amount: f32) -> Transaction {
        Transaction {
            client_id,
            transaction_id,
            kind: TransactionKind::Withdrawal {
                amount: OrderedFloat(amount),
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
}
