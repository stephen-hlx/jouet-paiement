use std::io::Read;

use csv::Trim;

use super::{TransactionRecordConsumer, TransactionStreamProcessError, TransactionStreamProcessor};

struct CsvStreamProcessor {
    consumer: Box<dyn TransactionRecordConsumer>,
}

impl TransactionStreamProcessor for CsvStreamProcessor {
    fn process<R: Read>(&mut self, r: R) -> Result<(), TransactionStreamProcessError> {
        let mut rdr = csv::ReaderBuilder::new().trim(Trim::All).from_reader(r);
        for result in rdr.deserialize() {
            match result {
                Ok(it) => self.consumer.consume(it)?,
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
    use rstest::rstest;

    use crate::{
        model::{ClientId, TransactionId},
        transaction_stream_processor::{
            TransactionRecord, TransactionRecordConsumer, TransactionRecordConsumerError,
            TransactionRecordType::{self, Chargeback, Deposit, Dispute, Resolve, Withdrawal},
            TransactionStreamProcessError, TransactionStreamProcessor,
        },
    };

    use super::CsvStreamProcessor;

    struct RecordSink {
        records: Rc<RefCell<Vec<TransactionRecord>>>,
    }

    impl TransactionRecordConsumer for RecordSink {
        fn consume(
            &mut self,
            transaction_record: TransactionRecord,
        ) -> Result<(), TransactionRecordConsumerError> {
            self.records.borrow_mut().push(transaction_record);
            Ok(())
        }
    }

    #[rstest]
    #[case("
    type,    client, tx, amount
    deposit,      1,  2,    3.0",
            vec![transaction(Deposit, 1, 2, Some(3.0))])]
    #[case("
    type,       client, tx, amount
    withdrawal,      4,  5,    6.0",
            vec![transaction(Withdrawal, 4, 5, Some(6.0))])]
    #[case("
    type,    client, tx, amount
    dispute,      7,  8,       ",
            vec![transaction(Dispute, 7, 8, None)])]
    #[case("
    type,    client, tx, amount
    resolve,      9, 10,       ",
            vec![transaction(Resolve, 9, 10, None)])]
    #[case("
    type,       client, tx, amount
    chargeback,     11, 12,       ",
            vec![transaction(Chargeback, 11, 12, None)])]
    #[case("
    type, client, tx, amount
    deposit, 1, 2, 3.0
    withdrawal, 4, 5, 6.0
    dispute, 7, 8,
    resolve, 9, 10,
    chargeback, 11, 12,",
            vec![transaction(Deposit, 1, 2, Some(3.0)),
            transaction(Withdrawal, 4, 5, Some(6.0)),
            transaction(Dispute, 7, 8, None),
            transaction(Resolve, 9, 10, None),
            transaction(Chargeback, 11, 12, None)])]
    fn csv_parser_works(#[case] input: &str, #[case] expected: Vec<TransactionRecord>) {
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
    impl TransactionRecordConsumer for Blackhole {
        fn consume(
            &mut self,
            _transaction_record: TransactionRecord,
        ) -> Result<(), TransactionRecordConsumerError> {
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

    fn transaction(
        txn_type: TransactionRecordType,
        client_id: ClientId,
        transaction_id: TransactionId,
        optional_amount: Option<f32>,
    ) -> TransactionRecord {
        TransactionRecord {
            txn_type,
            client_id,
            transaction_id,
            optional_amount,
        }
    }
}
