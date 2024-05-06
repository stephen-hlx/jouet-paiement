use std::{fs::File, io::BufReader, sync::Arc};

use dashmap::DashMap;
use jouet_paiement::{
    account::SimpleAccountTransactor,
    model::{AccountSummary, AccountSummaryCsvWriter},
    transaction_processor::SimpleTransactionProcessor,
    transaction_stream_processor::{
        async_csv_stream_processor::AsyncCsvStreamProcessor, TransactionStreamProcessor,
    },
};

#[tokio::test]
async fn e2e_small_input_using_async_processor() {
    let accounts = Arc::new(DashMap::new());
    let account_transaction_processor = SimpleAccountTransactor::new();
    let transaction_processor =
        SimpleTransactionProcessor::new(accounts.clone(), Box::new(account_transaction_processor));
    let senders_and_handles = DashMap::new();

    let processor =
        AsyncCsvStreamProcessor::new(Arc::new(transaction_processor), senders_and_handles);

    let file = File::open("tests/small_input.txt").unwrap();
    let reader = BufReader::new(file);

    processor.process(reader).await.unwrap();
    processor.shutdown().await.unwrap();

    let mut summaries: Vec<AccountSummary> =
        accounts.iter().map(|entry| entry.value().into()).collect();
    summaries.sort_by(|a, b| {
        a.client_id
            .partial_cmp(&b.client_id)
            .expect("ClientId is not a float so there is no way this could return a `None`.")
    });
    assert_eq!(
        String::from_utf8(AccountSummaryCsvWriter::write(summaries).unwrap()).unwrap(),
        "\
        client,available,held,total,locked\n\
        1,9.0000,0.0000,9.0000,false\n\
        2,6.0000,0.0000,6.0000,false\n"
    );
}
