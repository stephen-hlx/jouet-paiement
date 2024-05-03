use std::io::Read;

use csv::Trim;

use super::{
    to_transaction, TransactionConsumer, TransactionStreamProcessError, TransactionStreamProcessor,
};

struct CsvStreamProcessor {
    consumer: Box<dyn TransactionConsumer>,
}

impl TransactionStreamProcessor for CsvStreamProcessor {
    fn process<R: Read>(&mut self, r: R) -> Result<(), TransactionStreamProcessError> {
        let mut rdr = csv::ReaderBuilder::new().trim(Trim::All).from_reader(r);
        for result in rdr.deserialize() {
            match result {
                Ok(it) => self.consumer.consume(to_transaction(it)?)?,
                Err(err) => {
                    return Err(TransactionStreamProcessError::ParsingError(err.to_string()))
                }
            };
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use assert_matches::assert_matches;
    use ordered_float::OrderedFloat;
    use rstest::rstest;

    use crate::{
        model::{ClientId, TransactionId},
        transaction_processor::{Transaction, TransactionKind},
        transaction_stream_processor::{
            TransactionConsumer, TransactionConsumerError, TransactionStreamProcessError,
            TransactionStreamProcessor,
        },
    };

    use super::CsvStreamProcessor;

    struct RecordSink {
        records: Rc<RefCell<Vec<Transaction>>>,
    }

    impl TransactionConsumer for RecordSink {
        fn consume(&mut self, transaction: Transaction) -> Result<(), TransactionConsumerError> {
            self.records.borrow_mut().push(transaction);
            Ok(())
        }
    }

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
    fn csv_parser_works(#[case] input: &str, #[case] expected: Vec<Transaction>) {
        let records = Rc::new(RefCell::new(Vec::new()));
        let record_sink = RecordSink {
            records: records.clone(),
        };
        let mut processor = CsvStreamProcessor {
            consumer: Box::new(record_sink),
        };
        processor.process(input.as_bytes()).unwrap();
        assert_eq!(*records.borrow(), expected);
    }

    struct Blackhole;
    impl TransactionConsumer for Blackhole {
        fn consume(&mut self, _transaction: Transaction) -> Result<(), TransactionConsumerError> {
            Ok(())
        }
    }

    #[test]
    fn missing_coma_for_the_optional_field_results_in_parsing_error() {
        let input = "
    type,    client, tx, amount
    dispute,      7,  8";
        let blackhold = Blackhole;
        let mut processor = CsvStreamProcessor {
            consumer: Box::new(blackhold),
        };
        assert_matches!(
            processor.process(input.as_bytes()),
            Err(TransactionStreamProcessError::ParsingError(_))
        );
    }

    fn deposit(client_id: ClientId, transaction_id: TransactionId, amount: f32) -> Transaction {
        Transaction {
            client_id,
            transaction_id,
            kind: TransactionKind::DepositTransaction {
                amount: OrderedFloat(amount),
            },
        }
    }

    fn withdrawal(client_id: ClientId, transaction_id: TransactionId, amount: f32) -> Transaction {
        Transaction {
            client_id,
            transaction_id,
            kind: TransactionKind::WithdrawalTransaction {
                amount: OrderedFloat(amount),
            },
        }
    }

    fn dispute(client_id: ClientId, transaction_id: TransactionId) -> Transaction {
        Transaction {
            client_id,
            transaction_id,
            kind: TransactionKind::DisputeTransaction,
        }
    }

    fn resolve(client_id: ClientId, transaction_id: TransactionId) -> Transaction {
        Transaction {
            client_id,
            transaction_id,
            kind: TransactionKind::ResolveTransaction,
        }
    }

    fn chargeback(client_id: ClientId, transaction_id: TransactionId) -> Transaction {
        Transaction {
            client_id,
            transaction_id,
            kind: TransactionKind::ChargeBackTransaction,
        }
    }
}
