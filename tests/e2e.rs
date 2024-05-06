use std::{
    fs::File,
    io::{BufReader, BufWriter},
    sync::Arc,
};

use csv::WriterBuilder;
use dashmap::DashMap;
use jouet_paiement::{
    account::SimpleAccountTransactor,
    model::{AccountSummary, AccountSummaryCsvWriter},
    transaction_processor::SimpleTransactionProcessor,
    transaction_stream_processor::{
        async_csv_stream_processor::AsyncCsvStreamProcessor,
        csv_stream_processor::CsvStreamProcessor, TransactionRecord,
        TransactionRecordType::Deposit, TransactionStreamProcessor,
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

#[tokio::test(flavor = "multi_thread", worker_threads = 16)]
#[ignore = "this test takes time to run and should be enabled ondemand"]
async fn e2e_large_input_using_async_processor() {
    let accounts = Arc::new(DashMap::new());
    let account_transaction_processor = SimpleAccountTransactor::new();
    let transaction_processor =
        SimpleTransactionProcessor::new(accounts.clone(), Box::new(account_transaction_processor));
    let senders_and_handles = DashMap::new();

    let processor =
        AsyncCsvStreamProcessor::new(Arc::new(transaction_processor), senders_and_handles);

    create_test_file("/tmp/large_input.txt", create_test_records(10, 1_000_000));
    let file = File::open("/tmp/large_input.txt").unwrap();
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
        1,1000000.0000,0.0000,1000000.0000,false\n\
        2,1000000.0000,0.0000,1000000.0000,false\n\
        3,1000000.0000,0.0000,1000000.0000,false\n\
        4,1000000.0000,0.0000,1000000.0000,false\n\
        5,1000000.0000,0.0000,1000000.0000,false\n\
        6,1000000.0000,0.0000,1000000.0000,false\n\
        7,1000000.0000,0.0000,1000000.0000,false\n\
        8,1000000.0000,0.0000,1000000.0000,false\n\
        9,1000000.0000,0.0000,1000000.0000,false\n\
        10,1000000.0000,0.0000,1000000.0000,false\n"
    );
}

#[tokio::test]
#[ignore = "this test takes time to run and should be enabled ondemand"]
async fn e2e_large_input_using_blocking_processor() {
    let accounts = Arc::new(DashMap::new());
    let account_transaction_processor = SimpleAccountTransactor::new();
    let transaction_processor =
        SimpleTransactionProcessor::new(accounts.clone(), Box::new(account_transaction_processor));

    let processor = CsvStreamProcessor::new(Box::new(transaction_processor));

    create_test_file("/tmp/large_input.txt", create_test_records(10, 1_000_000));
    let file = File::open("/tmp/large_input.txt").unwrap();
    let reader = BufReader::new(file);

    processor.process(reader).await.unwrap();

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
        1,1000000.0000,0.0000,1000000.0000,false\n\
        2,1000000.0000,0.0000,1000000.0000,false\n\
        3,1000000.0000,0.0000,1000000.0000,false\n\
        4,1000000.0000,0.0000,1000000.0000,false\n\
        5,1000000.0000,0.0000,1000000.0000,false\n\
        6,1000000.0000,0.0000,1000000.0000,false\n\
        7,1000000.0000,0.0000,1000000.0000,false\n\
        8,1000000.0000,0.0000,1000000.0000,false\n\
        9,1000000.0000,0.0000,1000000.0000,false\n\
        10,1000000.0000,0.0000,1000000.0000,false\n"
    );
}

fn create_test_records(client_count: u16, transaction_count: u32) -> Vec<TransactionRecord> {
    let mut records = Vec::new();
    let mut transaction_id = 1u32;
    for _ in 1..=transaction_count {
        for client_id in 1..=client_count {
            records.push(TransactionRecord {
                txn_type: Deposit,
                client_id,
                transaction_id,
                optional_amount: Some("1".to_string()),
            });
            transaction_id += 1;
        }
    }
    records
}

fn create_test_file(filename: &str, records: Vec<TransactionRecord>) {
    let file = File::create(filename).unwrap();
    let buf_writer = BufWriter::new(file);
    let mut wtr = WriterBuilder::new().from_writer(buf_writer);
    for record in records {
        wtr.serialize(record).unwrap();
    }
    wtr.flush().unwrap();
}
