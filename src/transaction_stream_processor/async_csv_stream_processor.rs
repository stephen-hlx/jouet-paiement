use std::{io::Read, sync::Arc};

use async_trait::async_trait;
use csv::Trim;
use dashmap::DashMap;
use tokio::{
    sync::mpsc::{channel, Sender},
    task::JoinHandle,
};

use crate::{
    model::ClientId,
    transaction_processor::{Transaction, TransactionProcessor, TransactionProcessorError},
};

use super::{
    transaction_record_converter::to_transaction, TransactionStreamProcessError,
    TransactionStreamProcessor,
};

pub struct AsyncCsvStreamProcessor {
    transaction_processor: Arc<dyn TransactionProcessor + Send + Sync>,
    senders_and_handles: DashMap<
        ClientId,
        (
            Sender<Transaction>,
            JoinHandle<Result<(), TransactionProcessorError>>,
        ),
    >,
}

#[async_trait]
impl TransactionStreamProcessor for AsyncCsvStreamProcessor {
    async fn process(&self, r: impl Read + Send) -> Result<(), TransactionStreamProcessError> {
        let mut rdr = csv::ReaderBuilder::new().trim(Trim::All).from_reader(r);
        for result in rdr.deserialize() {
            match result {
                Ok(it) => self.do_process(to_transaction(it)?).await?,
                Err(err) => {
                    return Err(TransactionStreamProcessError::ParsingError(err.to_string()))
                }
            };
        }
        Ok(())
    }
}

impl AsyncCsvStreamProcessor {
    async fn do_process(
        &self,
        transaction: Transaction,
    ) -> Result<(), TransactionStreamProcessError> {
        let client_id = transaction.client_id;
        let binding = self
            .senders_and_handles
            .entry(client_id)
            .or_insert_with(|| self.create_channel());
        let sender = &binding.0;
        sender.send(transaction).await.unwrap();
        Ok(())
    }

    fn create_channel(
        &self,
    ) -> (
        Sender<Transaction>,
        JoinHandle<Result<(), TransactionProcessorError>>,
    ) {
        // TODO: make this configurable
        let (sender, mut receiver) = channel::<Transaction>(256);
        let clone = self.transaction_processor.clone();
        let handle = tokio::spawn(async move {
            while let Some(transaction) = receiver.recv().await {
                return clone.process(transaction).await;
            }
            Ok(())
        });
        (sender, handle)
    }

    pub fn new(
        consumer: Arc<dyn TransactionProcessor + Send + Sync>,
        senders_and_handles: DashMap<
            ClientId,
            (
                Sender<Transaction>,
                JoinHandle<Result<(), TransactionProcessorError>>,
            ),
        >,
    ) -> Self {
        Self {
            transaction_processor: consumer,
            senders_and_handles,
        }
    }

    pub async fn shutdown(self) -> Result<(), TransactionStreamProcessError> {
        for (_, (sender, handle)) in self.senders_and_handles {
            drop(sender);
            match handle.await {
                Ok(_) => {}
                Err(e) => {
                    return Err(TransactionStreamProcessError::FailedToShutdown(
                        e.to_string(),
                    ))
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use assert_matches::assert_matches;

    use dashmap::DashMap;

    use crate::transaction_processor::Blackhole;
    use crate::transaction_stream_processor::async_csv_stream_processor::AsyncCsvStreamProcessor;
    use crate::transaction_stream_processor::{
        TransactionStreamProcessError, TransactionStreamProcessor,
    };

    #[tokio::test]
    async fn missing_coma_for_the_optional_field_results_in_parsing_error() {
        let input = "
    type,    client, tx, amount
    dispute,      7,  8";
        let blackhole = Blackhole;
        let processor = AsyncCsvStreamProcessor::new(Arc::new(blackhole), DashMap::new());
        assert_matches!(
            processor.process(input.as_bytes()).await,
            Err(TransactionStreamProcessError::ParsingError(_))
        );
        processor.shutdown().await.unwrap();
    }
}
