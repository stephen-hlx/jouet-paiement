use std::io::Read;

use csv::Trim;

use crate::transaction_processor::TransactionProcessor;

use super::{to_transaction, TransactionStreamProcessError, TransactionStreamProcessor};

pub struct CsvStreamProcessor {
    consumer: Box<dyn TransactionProcessor>,
}

impl TransactionStreamProcessor for CsvStreamProcessor {
    fn process<R: Read>(&self, r: R) -> Result<(), TransactionStreamProcessError> {
        let mut rdr = csv::ReaderBuilder::new().trim(Trim::All).from_reader(r);
        for result in rdr.deserialize() {
            match result {
                Ok(it) => self.consumer.process(to_transaction(it)?)?,
                Err(err) => {
                    return Err(TransactionStreamProcessError::ParsingError(err.to_string()))
                }
            };
        }
        Ok(())
    }
}

impl CsvStreamProcessor {
    pub fn new(consumer: Box<dyn TransactionProcessor>) -> Self {
        Self { consumer }
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
        transaction_processor::{
            Transaction, TransactionKind, TransactionProcessor, TransactionProcessorError,
        },
        transaction_stream_processor::{TransactionStreamProcessError, TransactionStreamProcessor},
    };

    use super::CsvStreamProcessor;

    struct RecordSink {
        records: Rc<RefCell<Vec<Transaction>>>,
    }

    impl TransactionProcessor for RecordSink {
        fn process(&self, transaction: Transaction) -> Result<(), TransactionProcessorError> {
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
        let processor = CsvStreamProcessor {
            consumer: Box::new(record_sink),
        };
        processor.process(input.as_bytes()).unwrap();
        assert_eq!(*records.borrow(), expected);
    }

    struct Blackhole;
    impl TransactionProcessor for Blackhole {
        fn process(&self, _transaction: Transaction) -> Result<(), TransactionProcessorError> {
            Ok(())
        }
    }

    #[test]
    fn missing_coma_for_the_optional_field_results_in_parsing_error() {
        let input = "
    type,    client, tx, amount
    dispute,      7,  8";
        let blackhold = Blackhole;
        let processor = CsvStreamProcessor {
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
