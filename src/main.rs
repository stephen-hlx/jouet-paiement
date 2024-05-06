use std::{env, fs::File, io::BufReader, sync::Arc};

use dashmap::DashMap;
use jouet_paiement::account::SimpleAccountTransactor;
use jouet_paiement::model::{AccountSummary, AccountSummaryCsvWriter};
use jouet_paiement::transaction_processor::SimpleTransactionProcessor;
use jouet_paiement::transaction_stream_processor::async_csv_stream_processor::AsyncCsvStreamProcessor;
use jouet_paiement::transaction_stream_processor::TransactionStreamProcessor;
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
    let result = process_file(filename).await;
    println!("{result}");
}

async fn process_file(filename: &str) -> String {
    let accounts = Arc::new(DashMap::new());
    let account_transaction_processor = SimpleAccountTransactor::new();
    let transaction_processor =
        SimpleTransactionProcessor::new(accounts.clone(), Box::new(account_transaction_processor));
    let senders_and_handles = DashMap::new();

    let processor =
        AsyncCsvStreamProcessor::new(Arc::new(transaction_processor), senders_and_handles);
    let file = File::open(filename).unwrap();

    processor.process(BufReader::new(file)).await.unwrap();
    processor.shutdown().await.unwrap();
    let summaries: Vec<AccountSummary> = accounts
        .iter()
        .map(|entry| AccountSummary::from(entry.value()))
        .collect();
    String::from_utf8(AccountSummaryCsvWriter::write(summaries).unwrap()).unwrap()
}
