use std::io::Read;

use async_trait::async_trait;
use csv::Trim;

use crate::transaction_processor::TransactionProcessor;

use super::{
    error_handler::SimpleErrorHandler, transaction_record_converter::to_transaction, ErrorHandler,
    TransactionStreamProcessError, TransactionStreamProcessor,
};

pub struct CsvStreamProcessor {
    consumer: Box<dyn TransactionProcessor + Send + Sync>,
    error_handler: Box<dyn ErrorHandler + Send + Sync>,
}

#[async_trait]
impl TransactionStreamProcessor for CsvStreamProcessor {
    async fn process(&self, r: impl Read + Send) -> Result<(), TransactionStreamProcessError> {
        let mut rdr = csv::ReaderBuilder::new().trim(Trim::All).from_reader(r);
        for result in rdr.deserialize() {
            match result {
                Ok(it) => match self.consumer.process(to_transaction(it)?).await {
                    Ok(_) => {}
                    Err(err) => self.error_handler.handle(err)?,
                },
                Err(err) => {
                    return Err(TransactionStreamProcessError::ParsingError(err.to_string()));
                }
            };
        }
        Ok(())
    }
}

impl CsvStreamProcessor {
    // This struct is an early stage of implementation.
    // It is only used in test code now.
    #[allow(dead_code)]
    pub fn new(consumer: Box<dyn TransactionProcessor + Send + Sync>) -> Self {
        let error_handler = SimpleErrorHandler;
        Self {
            consumer,
            error_handler: Box::new(error_handler),
        }
    }
}

#[cfg(test)]
mod tests {
    use assert_matches::assert_matches;

    use crate::{
        transaction_processor::Blackhole,
        transaction_stream_processor::{TransactionStreamProcessError, TransactionStreamProcessor},
    };

    use super::CsvStreamProcessor;

    #[tokio::test]
    async fn missing_coma_for_the_optional_field_results_in_parsing_error() {
        let input = "
    type,    client, tx, amount
    dispute,      7,  8";
        let blackhold = Blackhole;
        let processor = CsvStreamProcessor::new(Box::new(blackhold));

        assert_matches!(
            processor.process(input.as_bytes()).await,
            Err(TransactionStreamProcessError::ParsingError(_))
        );
    }
}
