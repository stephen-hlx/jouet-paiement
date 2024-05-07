use std::{
    env,
    fs::File,
    io::{BufReader, Read},
    sync::Arc,
};

use dashmap::DashMap;

use crate::{
    account::SimpleAccountTransactor,
    model::{AccountSummary, AccountSummaryCsvWriter},
    transaction_processor::SimpleTransactionProcessor,
    transaction_stream_processor::{
        async_csv_stream_processor::AsyncCsvStreamProcessor, TransactionStreamProcessor,
    },
};
#[cfg(test)]
use rstest_reuse;

mod account;
mod model;
mod transaction_processor;
mod transaction_stream_processor;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let filename = args.get(1).unwrap();
    let file = File::open(filename).unwrap();
    let reader = BufReader::new(file);

    let result = process_file(reader).await;
    println!("{result}");
}

async fn process_file(reader: impl Read + Send) -> String {
    let accounts = Arc::new(DashMap::new());

    let processor = AsyncCsvStreamProcessor::new(
        Arc::new(SimpleTransactionProcessor::new(
            accounts.clone(),
            Box::new(SimpleAccountTransactor::new()),
        )),
        DashMap::new(),
    );

    processor.process(reader).await.unwrap();
    processor.shutdown().await.unwrap();
    let summaries: Vec<AccountSummary> = accounts
        .iter()
        .map(|entry| AccountSummary::from(entry.value()))
        .collect();
    String::from_utf8(AccountSummaryCsvWriter::write(summaries).unwrap()).unwrap()
}
