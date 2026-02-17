pub mod txt_format {
    use crate::Parser;
    use std::error::Error;
    use std::fmt::{Display, Formatter};
    use std::io::{BufRead, BufReader, Read, Write};
    use std::str::FromStr;

    enum TransactionType {
        Deposit,
        Transfer,
        Withdrawal
    }

    impl FromStr for TransactionType {
        type Err = ();

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "DEPOSIT" => Ok(TransactionType::Deposit),
                "TRANSFER" => Ok(TransactionType::Transfer),
                "WITHDRAWAL" => Ok(TransactionType::Withdrawal),

                _ => Err(()),
            }
        }
    }

    enum TransactionStatus {
        Pending,
        Success,
        Failure
    }

    impl FromStr for TransactionStatus {
        type Err = ();

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "PENDING" => Ok(TransactionStatus::Pending),
                "FAILURE" => Ok(TransactionStatus::Failure),
                "SUCCESS" => Ok(TransactionStatus::Success),

                _ => Err(()),
            }
        }
    }

    struct YPBankTextRecord {
        id: u32,
        transaction_type: TransactionType,
        from_user_id: u32,
        to_user_id: u32,
        amount: u64,
        timestamp: u64,
        transaction_status: TransactionStatus,
        description: String
    }

    #[derive(Debug)]
    enum TextRecordError {
        WrongLineFormat { line_index: usize },
        MissingColonAfterKey { line_index: usize },
        UnexpectedKey { line_index: usize },
        ReadLineError(std::io::Error),
        ParseError { wrong_field: String, line_index: usize },
        NotAllFieldsSet { line_index: usize },
        ExcessFields,
        WrongFieldsCount { line_index: usize },
        SourceIsEmpty
    }

    impl Display for TextRecordError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    impl Error for TextRecordError {}

    impl Parser<TextRecordError> for YPBankTextRecord {
        fn from_read<R: Read>(reader: &mut R) -> Result<Vec<YPBankTextRecord>, TextRecordError> {
            let mut records = vec![];
            let buff_reader = BufReader::new(reader);
            let mut current_transaction_raw: Vec<String> = vec![];

            for (index, line_result) in buff_reader.lines().enumerate() {
                let line = line_result.map_err(TextRecordError::ReadLineError)?;
                let trimmed_line = line.trim();

                if trimmed_line.starts_with('#') {
                    continue;
                }

                if trimmed_line.is_empty() {
                    if !current_transaction_raw.is_empty() {
                        let record = YPBankTextRecord::read_transaction(&current_transaction_raw, index)?;
                        records.push(record);
                        current_transaction_raw.clear();
                    }
                } else {
                    current_transaction_raw.push(trimmed_line.to_string());
                }
            }

            if !current_transaction_raw.is_empty() {
                let record = YPBankTextRecord::read_transaction(&current_transaction_raw, 0)?;
                records.push(record);
            }

            if records.is_empty() {
                Err(TextRecordError::SourceIsEmpty)
            } else {
                Ok(records)
            }
        }

        fn write_to<W: Write>(&mut self, writer: &mut W) -> Result<(), TextRecordError> {
            todo!()
        }
    }

    impl YPBankTextRecord {
        const FIELDS_COUNT: usize = 8;

        fn default() -> YPBankTextRecord {
            YPBankTextRecord {
                id: 0,
                transaction_type: TransactionType::Deposit,
                from_user_id: 0,
                to_user_id: 0,
                amount: 0,
                timestamp: 0,
                transaction_status: TransactionStatus::Pending,
                description: "".to_string()
            }
        }

        fn read_transaction(raw_data: &[String], start_line_index: usize) -> Result<Self, TextRecordError> {
            if raw_data.len() != Self::FIELDS_COUNT {
                return Err(TextRecordError::WrongFieldsCount { line_index: start_line_index });
            }

            let mut record = YPBankTextRecord::default();
            let line_index = start_line_index;

            for (offset, raw_string) in raw_data.iter().enumerate() {
                let line_index = start_line_index + offset;

                let (key, value) = raw_string
                    .split_once(':')
                    .ok_or(TextRecordError::WrongLineFormat { line_index })?;

                let key = key.trim();
                let value = value.trim();

                match key {
                    "TX_ID" => {
                        record.id = parse(key, value, line_index)?;
                    }
                    "TX_TYPE" => {
                        record.transaction_type = parse(key, value, line_index)?;
                    }
                    "FROM_USER_ID" => {
                        record.from_user_id = parse(key, value, line_index)?;
                    }
                    "TO_USER_ID" => {
                        record.to_user_id = parse(key, value, line_index)?;
                    }
                    "AMOUNT" => {
                        record.amount = parse(key, value, line_index)?;
                    }
                    "TIMESTAMP" => {
                        record.timestamp = parse(key, value, line_index)?;
                    }
                    "STATUS" => {
                        record.transaction_status = parse(key, value, line_index)?;
                    }
                    "DESCRIPTION" => {
                        record.description = parse(key, value, line_index)?;
                    }
                    _ => return Err(TextRecordError::UnexpectedKey { line_index }),
                }
            }

            Ok(record)
        }
    }

    fn parse<T: FromStr>(key: &str, value: &str, line_index: usize) -> Result<T, TextRecordError> {
        value.parse().or_else(|_| return Err(TextRecordError::ParseError { wrong_field: key.to_string(), line_index }))
    }
}