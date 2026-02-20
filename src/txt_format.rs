pub mod txt_format {
    use crate::Parser;
    use serde::Deserialize;
    use serde_with::{serde_as, DisplayFromStr};
    use std::collections::HashMap;
    use std::error::Error;
    use std::fmt::{Display, Formatter};
    use std::io::{BufRead, BufReader, BufWriter, Read, Write};
    use std::str::FromStr;

    #[derive(Debug, Deserialize, PartialEq)]
    enum TransactionType {
        #[serde(rename = "DEPOSIT")] Deposit,
        #[serde(rename = "TRANSFER")] Transfer,
        #[serde(rename = "WITHDRAWAL")] Withdrawal
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

    impl Display for TransactionType {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                TransactionType::Deposit => write!(f, "DEPOSIT"),
                TransactionType::Transfer => write!(f, "TRANSFER"),
                TransactionType::Withdrawal => write!(f, "WITHDRAWAL"),
            }
        }
    }

    #[derive(Debug, Deserialize, PartialEq)]
    enum TransactionStatus {
        #[serde(rename = "PENDING")] Pending,
        #[serde(rename = "SUCCESS")] Success,
        #[serde(rename = "FAILURE")] Failure
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

    impl Display for TransactionStatus {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                TransactionStatus::Pending => write!(f, "PENDING"),
                TransactionStatus::Success => write!(f, "SUCCESS"),
                TransactionStatus::Failure => write!(f, "FAILURE")
            }
        }
    }

    #[serde_as]
    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct YPBankTextRecord {
        #[serde(rename = "TX_ID")]
        #[serde_as(as = "DisplayFromStr")]
        id: u32,

        #[serde(rename = "TX_TYPE")]
        transaction_type: TransactionType,

        #[serde(rename = "FROM_USER_ID")]
        #[serde_as(as = "DisplayFromStr")]
        from_user_id: u32,

        #[serde(rename = "TO_USER_ID")]
        #[serde_as(as = "DisplayFromStr")]
        to_user_id: u32,

        #[serde(rename = "AMOUNT")]
        #[serde_as(as = "DisplayFromStr")]
        amount: u64,

        #[serde(rename = "TIMESTAMP")]
        #[serde_as(as = "DisplayFromStr")]
        timestamp: u64,

        #[serde(rename = "STATUS")]
        transaction_status: TransactionStatus,
        #[serde(rename = "DESCRIPTION")]
        description: String
    }

    #[derive(Debug)]
    enum TextRecordError {
        MissingColonAfterKey,
        ReadLineError(std::io::Error),
        ParseError { error: String },
        SourceIsEmpty,
        EmptyLinesAtEndOfFile
    }

    impl Display for TextRecordError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    impl Error for TextRecordError {}

    impl From<serde::de::value::Error> for TextRecordError {
        fn from(value: serde::de::value::Error) -> Self {
            TextRecordError::ParseError { error: value.to_string() }
        }
    }

    impl Parser<TextRecordError, std::io::Error> for YPBankTextRecord {
        fn from_read<R: Read>(reader: &mut R) -> Result<YPBankTextRecord, TextRecordError> {
            let mut kv_pairs: HashMap<String, String> = HashMap::with_capacity(8);
            let mut line_buf: Vec<u8> = Vec::with_capacity(128);
            let mut byte_buf = [0u8; 1];
            let mut has_started = false;

            loop {
                line_buf.clear();
                let mut eof = false;

                loop {
                    match reader.read(&mut byte_buf).map_err(TextRecordError::ReadLineError)? {
                        0 => { eof = true; break; }
                        _ if byte_buf[0] == b'\n' => { has_started = true; break; }
                        _ => {
                            has_started = true;
                            line_buf.push(byte_buf[0]);
                        }
                    }
                }

                let line_str = std::str::from_utf8(&line_buf)
                    .map_err(|_| TextRecordError::ParseError { error: "Invalid UTF-8".into() })?;

                let trimmed_line = line_str.trim();

                if trimmed_line.starts_with('#') {
                    continue;
                }

                if trimmed_line.is_empty() {
                    if !kv_pairs.is_empty() {
                        return Ok(Self::parse_transaction(&mut kv_pairs)?);
                    }

                    if eof {
                        return if !has_started {
                            Err(TextRecordError::SourceIsEmpty)
                        } else {
                            Err(TextRecordError::EmptyLinesAtEndOfFile)
                        };
                    }

                    continue;
                }

                let (k, v) = trimmed_line
                    .split_once(':')
                    .ok_or(TextRecordError::MissingColonAfterKey)?;

                kv_pairs.insert(k.trim().to_owned(), v.trim().to_owned());

                if eof && !kv_pairs.is_empty() {
                    return Ok(Self::parse_transaction(&mut kv_pairs)?);
                }
            }
        }

        /*fn from_read<R: Read>(reader: &mut R) -> Result<YPBankTextRecord, TextRecordError> {
            let mut buff_reader = BufReader::new(reader);
            let mut kv_pairs: HashMap<String, String> = HashMap::with_capacity(8); //сразу аллоцируем память
            let mut line_buf = String::with_capacity(128);

            if buff_reader.fill_buf()
                .map_err(|e| TextRecordError::ReadLineError(e))?
                .is_empty() {
                return Err(TextRecordError::SourceIsEmpty)
            }

            loop {
                match buff_reader.read_line(&mut line_buf) {
                    Ok(0) => break, //EOF
                    Ok(_) => {
                        let trimmed_line = line_buf.trim();

                        if trimmed_line.starts_with('#') {
                            line_buf.clear();
                            continue;
                        }

                        if trimmed_line.is_empty() {
                            if !kv_pairs.is_empty() {
                                line_buf.clear();

                                return Ok(Self::parse_transaction(&mut kv_pairs)?);
                            }
                        } else {
                            let (k, v) = trimmed_line
                                .split_once(':')
                                .ok_or(TextRecordError::MissingColonAfterKey)?;

                            kv_pairs.insert(k.trim().to_owned(), v.trim().to_owned());
                        }

                        line_buf.clear()
                    }
                    Err(e) => return Err(TextRecordError::ReadLineError(e)),
                }
            }

            // Обработка последнего блока
            if !kv_pairs.is_empty() {
                let res = Self::parse_transaction(&mut kv_pairs)?;
                kv_pairs.clear();

                return Ok(res);
            }

            Err(TextRecordError::EmptyLinesAtEndOfFile)
        }*/

        fn write_to<W: Write>(&mut self, writer: &mut W) -> Result<(), std::io::Error> {
            let mut buff_writer = BufWriter::new(writer);

            writeln!(&mut buff_writer, "TX_ID: {}", self.id)?;
            writeln!(&mut buff_writer, "TX_TYPE: {}", self.transaction_type)?;
            writeln!(&mut buff_writer, "FROM_USER_ID: {}", self.from_user_id)?;

            writeln!(&mut buff_writer, "TO_USER_ID: {}", self.to_user_id)?;
            writeln!(&mut buff_writer, "AMOUNT: {}", self.amount)?;
            writeln!(&mut buff_writer, "TIMESTAMP: {}", self.timestamp)?;

            writeln!(&mut buff_writer, "STATUS: {}", self.transaction_status)?;
            writeln!(&mut buff_writer, "DESCRIPTION: {}", self.description)?;
            writeln!(&mut buff_writer)?; // пустая строка как разделитель
            buff_writer.flush()?;
            Ok(())
        }
    }

    impl YPBankTextRecord {
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

        fn parse_transaction(map: &mut HashMap<String, String>) -> Result<Self, serde::de::value::Error> {
            Self::deserialize(serde::de::value::MapDeserializer::new(map.drain()))
                .map_err(|e: serde::de::value::Error| e)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::io::Cursor;

        fn sample_record() -> YPBankTextRecord {
            YPBankTextRecord {
                id: 1234567890,
                transaction_type: TransactionType::Transfer,
                from_user_id: 111,
                to_user_id: 222,
                amount: 1000,
                timestamp: 1633056800000,
                transaction_status: TransactionStatus::Failure,
                // по спецификации DESCRIPTION должен быть в двойных кавычках
                description: "\"User transfer\"".to_string(),
            }
        }

        #[test]
        fn write_to_writes_all_required_fields_and_blank_separator_line() {
            let mut rec = sample_record();

            // Пишем в in-memory поток, как и читаем из него в from_read-тестах
            let mut out = Cursor::new(Vec::<u8>::new());

            rec.write_to(&mut out).unwrap();

            let bytes = out.into_inner();
            let s = String::from_utf8(bytes).unwrap();

            // обязательные поля (по спецификации) должны присутствовать
            assert!(s.contains("TX_ID: 1234567890\n"));
            assert!(s.contains("TX_TYPE: TRANSFER\n"));
            assert!(s.contains("FROM_USER_ID: 111\n"));
            assert!(s.contains("TO_USER_ID: 222\n"));
            assert!(s.contains("AMOUNT: 1000\n"));
            assert!(s.contains("TIMESTAMP: 1633056800000\n"));
            assert!(s.contains("STATUS: FAILURE\n"));

            // DESCRIPTION пишется как есть; тест закрепляет требование кавычек из спецификации
            assert!(s.contains("DESCRIPTION: \"User transfer\"\n"));

            // запись должна заканчиваться пустой строкой-разделителем
            assert!(s.ends_with("\n\n"), "expected record to end with a blank line separator, got: {s:?}");
        }

        #[test]
        fn from_read_parses_record_with_arbitrary_field_order_and_ignores_comments() {
            let input = r#"
# leading comment
TX_ID: 2312321321
TIMESTAMP: 1633056800000
STATUS: FAILURE
TX_TYPE: TRANSFER
FROM_USER_ID: 123
TO_USER_ID: 987
AMOUNT: 1000
DESCRIPTION: "User transfer"

"#;

            let mut cur = Cursor::new(input.as_bytes());
            let rec = YPBankTextRecord::from_read(&mut cur).unwrap();

            assert_eq!(rec.id, 2312321321);
            assert_eq!(rec.transaction_type, TransactionType::Transfer);
            assert_eq!(rec.from_user_id, 123);
            assert_eq!(rec.to_user_id, 987);
            assert_eq!(rec.amount, 1000);
            assert_eq!(rec.timestamp, 1633056800000);
            assert_eq!(rec.transaction_status, TransactionStatus::Failure);
            assert_eq!(rec.description, "\"User transfer\"");
        }

        #[test]
        fn from_read_reads_two_records_separated_by_blank_line() {
            let input = r#"
# Record 1
TX_ID: 1
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 10
AMOUNT: 100
TIMESTAMP: 1
STATUS: SUCCESS
DESCRIPTION: "Terminal deposit"

# Record 2
TX_ID: 2
TX_TYPE: WITHDRAWAL
FROM_USER_ID: 10
TO_USER_ID: 0
AMOUNT: 50
TIMESTAMP: 2
STATUS: PENDING
DESCRIPTION: "User withdrawal"
"#;

            let mut cur = Cursor::new(input.as_bytes());

            let r1 = YPBankTextRecord::from_read(&mut cur).unwrap();
            let r2 = YPBankTextRecord::from_read(&mut cur).unwrap();

            // Record 1
            assert_eq!(r1.id, 1);
            assert_eq!(r1.transaction_type, TransactionType::Deposit);
            assert_eq!(r1.from_user_id, 0);
            assert_eq!(r1.to_user_id, 10);
            assert_eq!(r1.amount, 100);
            assert_eq!(r1.timestamp, 1);
            assert_eq!(r1.transaction_status, TransactionStatus::Success);
            assert_eq!(r1.description, "\"Terminal deposit\"");

            // Record 2
            assert_eq!(r2.id, 2);
            assert_eq!(r2.transaction_type, TransactionType::Withdrawal);
            assert_eq!(r2.from_user_id, 10);
            assert_eq!(r2.to_user_id, 0);
            assert_eq!(r2.amount, 50);
            assert_eq!(r2.timestamp, 2);
            assert_eq!(r2.transaction_status, TransactionStatus::Pending);
            assert_eq!(r2.description, "\"User withdrawal\"");
        }

        #[test]
        fn from_read_parses_last_block_without_trailing_blank_line() {
            let input = r#"
TX_ID: 3
TX_TYPE: TRANSFER
FROM_USER_ID: 1
TO_USER_ID: 2
AMOUNT: 7
TIMESTAMP: 3
STATUS: SUCCESS
DESCRIPTION: "No trailing blank"
"#;

            let mut cur = Cursor::new(input.as_bytes());
            let rec = YPBankTextRecord::from_read(&mut cur).unwrap();

            assert_eq!(rec.id, 3);
            assert_eq!(rec.description, "\"No trailing blank\"");
        }

        #[test]
        fn from_read_errors_on_line_without_colon() {
            let input = r#"
TX_ID 123
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 1
AMOUNT: 1
TIMESTAMP: 1
STATUS: SUCCESS
DESCRIPTION: "x"

"#;

            let mut cur = Cursor::new(input.as_bytes());
            let err = YPBankTextRecord::from_read(&mut cur).unwrap_err();
            assert!(matches!(err, TextRecordError::MissingColonAfterKey));
        }

        #[test]
        fn from_read_errors_on_invalid_data_types() {
            // 1. Отрицательный ID (ожидается u32)
            let input_neg_id = r#"
TX_ID: -5
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 10
AMOUNT: 100
TIMESTAMP: 1
STATUS: SUCCESS
DESCRIPTION: "Negative ID"
"#;
            let mut cur = Cursor::new(input_neg_id.as_bytes());
            let err = YPBankTextRecord::from_read(&mut cur).unwrap_err();
            assert!(matches!(err, TextRecordError::ParseError { .. }));

            // 2. Строка вместо числа в AMOUNT
            let input_bad_amount = r#"
TX_ID: 10
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 10
AMOUNT: not_a_number
TIMESTAMP: 1
STATUS: SUCCESS
DESCRIPTION: "Bad Amount"
"#;
            let mut cur = Cursor::new(input_bad_amount.as_bytes());
            let err = YPBankTextRecord::from_read(&mut cur).unwrap_err();
            assert!(matches!(err, TextRecordError::ParseError { .. }));

            // 3. Некорректный статус транзакции
            let input_bad_status = r#"
TX_ID: 11
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 10
AMOUNT: 100
TIMESTAMP: 1
STATUS: UNKNOWN_STATUS
DESCRIPTION: "Bad Status"
"#;
            let mut cur = Cursor::new(input_bad_status.as_bytes());
            let err = YPBankTextRecord::from_read(&mut cur).unwrap_err();
            assert!(matches!(err, TextRecordError::ParseError { .. }));
        }

        #[test]
        fn from_read_errors_on_empty_source() {
            let input = "";
            let mut cur = Cursor::new(input.as_bytes());
            let err = YPBankTextRecord::from_read(&mut cur).unwrap_err();

            assert!(matches!(err, TextRecordError::SourceIsEmpty));
        }

        #[test]
        fn from_read_ignores_extra_fields() {
            let input = r#"
TX_ID: 999
TX_TYPE: DEPOSIT
FROM_USER_ID: 10
TO_USER_ID: 20
AMOUNT: 500
TIMESTAMP: 123456789
STATUS: SUCCESS
DESCRIPTION: "Extra fields test"
UNKNOWN_FIELD: some_value
ANOTHER_ONE: 123
"#;
            let mut cur = Cursor::new(input.as_bytes());
            let Err(_) = YPBankTextRecord::from_read(&mut cur) else {
                panic!("Extra field skipped.")
            };
        }
    }
}