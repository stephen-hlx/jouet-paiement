use csv::WriterBuilder;
use thiserror::Error;

use crate::account::{Account, AccountSnapshot, AccountStatus};

use super::{AccountSummary, Amount4DecimalBased};

impl From<Account> for AccountSummary {
    fn from(account: Account) -> Self {
        AccountSummary::from(&account)
    }
}

impl From<&Account> for AccountSummary {
    fn from(account: &Account) -> Self {
        let AccountSnapshot { available, held } = account.account_snapshot;
        let total = Amount4DecimalBased(available.0 + held.0);
        Self {
            client_id: account.client_id,
            available: available.to_str(),
            held: held.to_str(),
            total: total.to_str(),
            locked: account.status == AccountStatus::Locked,
        }
    }
}

pub struct AccountSummaryCsvWriter;

#[derive(Debug, Error)]
pub enum AccountSummaryWriterError {
    #[error("Failed to serialise the AccountSummary: {0}")]
    SerialisationError(String),
}

impl AccountSummaryCsvWriter {
    pub fn write(summaries: Vec<AccountSummary>) -> Result<Vec<u8>, AccountSummaryWriterError> {
        let mut wtr = WriterBuilder::new().from_writer(vec![]);
        for summary in summaries {
            match wtr.serialize(summary) {
                Ok(_) => {}
                Err(err) => {
                    return Err(AccountSummaryWriterError::SerialisationError(
                        err.to_string(),
                    ))
                }
            };
        }
        match wtr.into_inner() {
            Ok(chars) => Ok(chars),
            Err(e) => return Err(AccountSummaryWriterError::SerialisationError(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::model::AccountSummary;

    use super::AccountSummaryCsvWriter;

    #[test]
    fn can_write_account_summary_data_as_csv() {
        let account_summary_1 = AccountSummary {
            client_id: 1122,
            available: "111".to_string(),
            held: "222".to_string(),
            total: "333".to_string(),
            locked: false,
        };
        let account_summary_2 = AccountSummary {
            client_id: 3344,
            available: "333".to_string(),
            held: "444".to_string(),
            total: "777".to_string(),
            locked: true,
        };

        assert_eq!(
            String::from_utf8(
                AccountSummaryCsvWriter::write(vec![account_summary_1, account_summary_2]).unwrap()
            )
            .unwrap(),
            "\
            client,available,held,total,locked\n\
            1122,111,222,333,false\n\
            3344,333,444,777,true\n"
        );
    }
}
