use std::{collections::HashMap, sync::Arc};

use dashmap::DashMap;
use jouet_paiement::{
    account::{
        Account, AccountSnapshot, AccountStatus::Active, Deposit, DepositStatus::Accepted,
        SimpleAccountTransactionProcessor, Withdrawal,
    },
    model::{ClientId, TransactionId},
    transaction_processor::SimpleTransactionProcessor,
    transaction_stream_processor::{
        csv_stream_processor::CsvStreamProcessor, TransactionStreamProcessor,
    },
};
use ordered_float::OrderedFloat;

#[tokio::test]
async fn e2e_small_input() {
    let accounts = Arc::new(DashMap::new());
    let account_transaction_processor = SimpleAccountTransactionProcessor::new();
    let transaction_consumer =
        SimpleTransactionProcessor::new(accounts.clone(), Box::new(account_transaction_processor));
    let csv_stream_processor = CsvStreamProcessor::new(Box::new(transaction_consumer));

    let input = "
       type, client, tx, amount
    deposit,      1, 10,    4.0
    deposit,      1, 20,    5.0
    deposit,      2, 30,    6.0";

    let mut client_1_deposits = HashMap::new();
    client_1_deposits.insert(10, accepted_deposit(4.0));
    client_1_deposits.insert(20, accepted_deposit(5.0));

    let mut client_2_deposits = HashMap::new();
    client_2_deposits.insert(30, accepted_deposit(6.0));

    let mut expected_accounts = HashMap::new();
    expected_accounts.insert(
        1,
        active_account(1, snapshot(9, 0), client_1_deposits, HashMap::new()),
    );
    expected_accounts.insert(
        2,
        active_account(2, snapshot(6, 0), client_2_deposits, HashMap::new()),
    );

    csv_stream_processor
        .process(input.as_bytes())
        .await
        .unwrap();
    assert_eq!(accounts.len(), expected_accounts.len());
    accounts.iter().for_each(|entry| {
        let key = entry.key();
        let value = entry.value();
        assert_eq!(value, expected_accounts.get(key).unwrap());
    });
}

fn snapshot(available: i32, held: u32) -> AccountSnapshot {
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

fn accepted_deposit(amount: f32) -> Deposit {
    Deposit {
        amount: OrderedFloat(amount),
        status: Accepted,
    }
}
