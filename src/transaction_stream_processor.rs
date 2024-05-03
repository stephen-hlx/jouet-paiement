use std::io;

use serde::Deserialize;
use thiserror::Error;

use crate::model::{ClientId, TransactionId};

trait TransactionStreamProcessor {
    fn process<R: io::Read>(&mut self, r: R) -> Result<(), TransactionStreamProcessError>;
}

#[derive(Debug, Error)]
enum TransactionStreamProcessError {
    #[error("Error occurred during parsing the input data: {0}")]
    ParsingError(String),
    #[error("Error occurred during processing the `TransactionRecord` {0:?} due to {1}")]
    ProcessError(TransactionRecord, String),
}

#[derive(Debug, Deserialize, PartialEq)]
struct TransactionRecord {
    #[serde(rename = "type")]
    txn_type: String,
    #[serde(rename = "client")]
    client_id: ClientId,
    #[serde(rename = "tx")]
    transaction_id: TransactionId,
    #[serde(rename = "amount")]
    optional_amount: Option<f32>,
}

mod csv_stream_processor;

trait TransactionRecordConsumer {
    fn consume(
        &mut self,
        transaction_record: TransactionRecord,
    ) -> Result<(), TransactionRecordConsumerError>;
}

#[derive(Debug, Error)]
#[error("Failed to consumer the `TransactionRecord` {0:?} due to {1}")]
struct TransactionRecordConsumerError(TransactionRecord, String);

impl From<TransactionRecordConsumerError> for TransactionStreamProcessError {
    fn from(value: TransactionRecordConsumerError) -> Self {
        Self::ProcessError(value.0, value.1)
    }
}
