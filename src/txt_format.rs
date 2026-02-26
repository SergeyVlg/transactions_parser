use crate::common::{Transaction, TransactionStatus, TransactionType};
use crate::{IsEofError, Readable, Writable};
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};

//noinspection DuplicatedCode
/// Запись транзакции в текстовом формате "ключ-значение".
///
/// Каждая запись состоит из набора строк вида `КЛЮЧ: ЗНАЧЕНИЕ`.
/// Записи разделяются одной или несколькими пустыми строками.
/// Комментарии начинаются с символа `#`.
#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YPBankTextRecord {
    #[serde(rename = "TX_ID")]
    #[serde_as(as = "DisplayFromStr")]
    id: u64,

    #[serde(rename = "TX_TYPE")]
    transaction_type: TransactionType,

    #[serde(rename = "FROM_USER_ID")]
    #[serde_as(as = "DisplayFromStr")]
    from_user_id: u64,

    #[serde(rename = "TO_USER_ID")]
    #[serde_as(as = "DisplayFromStr")]
    to_user_id: u64,

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

/// Ошибки, возникающие при парсинге текстовых записей.
#[derive(Debug)]
pub enum TextRecordError {
    /// Отсутствует двоеточие, разделяющее ключ и значение.
    MissingColonAfterKey,
    /// Ошибка ввода-вывода при чтении строки.
    ReadLineError(std::io::Error),
    /// Ошибка парсинга полей (например, неверный формат числа или даты).
    ParseError { error: String },
    /// Достигнут конец файла.
    EndOfFile,
}

impl Display for TextRecordError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for TextRecordError {}

impl IsEofError for TextRecordError {
    fn is_eof(&self) -> bool {
        matches!(self, TextRecordError::EndOfFile)
    }
}

impl From<std::io::Error> for TextRecordError {
    fn from(value: std::io::Error) -> Self {
        TextRecordError::ReadLineError(value)
    }
}

impl From<serde::de::value::Error> for TextRecordError {
    fn from(value: serde::de::value::Error) -> Self {
        TextRecordError::ParseError { error: value.to_string() }
    }
}

impl From<TextRecordError> for std::io::Error {
    fn from(value: TextRecordError) -> Self {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, value)
    }
}

//noinspection DuplicatedCode
impl From<YPBankTextRecord> for Transaction {
    fn from(value: YPBankTextRecord) -> Self {
        Transaction {
            id: value.id,
            transaction_type: value.transaction_type,
            from_user_id: value.from_user_id,
            to_user_id: value.to_user_id,
            amount: value.amount as i64,
            timestamp: value.timestamp,
            transaction_status: value.transaction_status,
            description: value.description,
        }
    }
}

impl From<Transaction> for YPBankTextRecord {
    fn from(value: Transaction) -> Self {
        YPBankTextRecord {
            id: value.id,
            transaction_type: value.transaction_type,
            from_user_id: value.from_user_id,
            to_user_id: value.to_user_id,
            amount: value.amount as u64,
            timestamp: value.timestamp,
            transaction_status: value.transaction_status,
            description: value.description,
        }
    }
}

impl<R: Read> Readable<R> for YPBankTextRecord {
    type Reader = BufReader<R>;
    type Error = TextRecordError;

    fn build_reader(source: R) -> Self::Reader {
        BufReader::new(source)
    }

    fn read(reader: &mut Self::Reader) -> Result<YPBankTextRecord, TextRecordError> {
        if reader.fill_buf()?.is_empty() {
            return Err(TextRecordError::EndOfFile);
        }

        let mut kv_pairs: HashMap<String, String> = HashMap::with_capacity(8);
        let mut line_buf = String::with_capacity(128);

        loop {
            line_buf.clear();
            let bytes_read = reader.read_line(&mut line_buf)?;

            if bytes_read == 0 { //EOF
                break;
            }

            let trimmed = line_buf.trim();
            if trimmed.starts_with('#') {
                continue;
            }

            if trimmed.is_empty() {
                if !kv_pairs.is_empty() {
                    return Ok(Self::parse_transaction(&mut kv_pairs)?);
                }

                continue;
            }

            let (k, v) = trimmed
                .split_once(':')
                .ok_or(TextRecordError::MissingColonAfterKey)?;

            kv_pairs.insert(k.trim().to_owned(), v.trim().trim_matches('"').to_owned());
        }

        if kv_pairs.is_empty() {
            Err(TextRecordError::EndOfFile)
        } else {
            Ok(Self::parse_transaction(&mut kv_pairs)?)
        }
    }
}

impl Writable for YPBankTextRecord {
    type Error = std::io::Error;

    fn write_header<W: Write>(_: &mut W) -> Result<(), Self::Error> {
        Ok(())
    }

    fn write<W: Write>(&self, writer: &mut W) -> Result<(), Self::Error> {
        let mut buff_writer = BufWriter::new(writer);

        writeln!(&mut buff_writer, "TX_ID: {}", self.id)?;
        writeln!(&mut buff_writer, "TX_TYPE: {}", self.transaction_type)?;
        writeln!(&mut buff_writer, "FROM_USER_ID: {}", self.from_user_id)?;

        writeln!(&mut buff_writer, "TO_USER_ID: {}", self.to_user_id)?;
        writeln!(&mut buff_writer, "AMOUNT: {}", self.amount)?;
        writeln!(&mut buff_writer, "TIMESTAMP: {}", self.timestamp)?;

        writeln!(&mut buff_writer, "STATUS: {}", self.transaction_status)?;
        writeln!(&mut buff_writer, "DESCRIPTION: \"{}\"", self.description)?;
        writeln!(&mut buff_writer)?;
        buff_writer.flush()?;
        Ok(())
    }
}

impl YPBankTextRecord {
    fn parse_transaction(map: &mut HashMap<String, String>) -> Result<Self, serde::de::value::Error> {
        Self::deserialize(serde::de::value::MapDeserializer::new(map.drain()))
            .map_err(|e: serde::de::value::Error| e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Parser, Serializer};
    use std::io::Cursor;

    #[test]
    fn writes_all_required_fields_and_blank_separator_line() {
        let rec1 = YPBankTextRecord {
            id: 1234567890,
            transaction_type: TransactionType::Transfer,
            from_user_id: 111,
            to_user_id: 222,
            amount: 1000,
            timestamp: 1633056800000,
            transaction_status: TransactionStatus::Failure,
            description: "User transfer".to_string(),
        };
        let rec2 = YPBankTextRecord {
            id: 999,
            transaction_type: TransactionType::Deposit,
            from_user_id: 555,
            to_user_id: 666,
            amount: 50,
            timestamp: 1700000000,
            transaction_status: TransactionStatus::Success,
            description: "Second tx".to_string(),
        };

        let writer = Cursor::new(Vec::<u8>::new());
        let mut serializer = Serializer::new(writer);

        serializer.serialize(vec![rec1, rec2]).unwrap();

        let bytes = serializer.into_inner().into_inner().unwrap().into_inner();
        let s = String::from_utf8(bytes).unwrap();

        assert!(s.contains("TX_ID: 1234567890\n"));
        assert!(s.contains("TX_TYPE: TRANSFER\n"));
        assert!(s.contains("DESCRIPTION: \"User transfer\"\n"));

        assert!(s.contains("TX_ID: 999\n"));
        assert!(s.contains("TX_TYPE: DEPOSIT\n"));
        assert!(s.contains("DESCRIPTION: \"Second tx\"\n"));

        let parts: Vec<&str> = s.split("\n\n").collect();
        assert_eq!(parts.len(), 3, "Should have 2 records separated by blank lines and trailing newline");
        assert!(!parts[0].is_empty());
        assert!(!parts[1].is_empty());
        assert!(parts[2].is_empty());

        assert!(s.ends_with("\n\n"), "expected output to end with a blank line separator");
    }

    #[test]
    fn read_parses_record_with_arbitrary_field_order_and_ignores_comments() {
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

        let cur = Cursor::new(input.as_bytes());
        let mut parser = Parser::<YPBankTextRecord, _>::new(cur);
        let rec = parser.next().expect("Should have a record");

        assert!(parser.next().is_none(), "Should be consumed");
        assert!(parser.read_error.is_none(), "Read error: {:?}", parser.read_error);

        assert_eq!(rec.id, 2312321321);
        assert_eq!(rec.transaction_type, TransactionType::Transfer);
        assert_eq!(rec.from_user_id, 123);
        assert_eq!(rec.to_user_id, 987);
        assert_eq!(rec.amount, 1000);
        assert_eq!(rec.timestamp, 1633056800000);
        assert_eq!(rec.transaction_status, TransactionStatus::Failure);
        assert_eq!(rec.description, "User transfer");
    }

    #[test]
    fn read_reads_two_records_separated_by_blank_line() {
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

        let cur = Cursor::new(input.as_bytes());
        let mut parser = Parser::<YPBankTextRecord, _>::new(cur);

        let r1 = parser.next().expect("Should have first record");
        let r2 = parser.next().expect("Should have second record");

        assert!(parser.next().is_none());
        assert!(parser.read_error.is_none(), "Read error: {}", parser.read_error.unwrap());

        assert_eq!(r1.id, 1);
        assert_eq!(r1.transaction_type, TransactionType::Deposit);
        assert_eq!(r1.from_user_id, 0);
        assert_eq!(r1.to_user_id, 10);
        assert_eq!(r1.amount, 100);
        assert_eq!(r1.timestamp, 1);
        assert_eq!(r1.transaction_status, TransactionStatus::Success);
        assert_eq!(r1.description, "Terminal deposit");

        assert_eq!(r2.id, 2);
        assert_eq!(r2.transaction_type, TransactionType::Withdrawal);
        assert_eq!(r2.from_user_id, 10);
        assert_eq!(r2.to_user_id, 0);
        assert_eq!(r2.amount, 50);
        assert_eq!(r2.timestamp, 2);
        assert_eq!(r2.transaction_status, TransactionStatus::Pending);
        assert_eq!(r2.description, "User withdrawal");
    }

    #[test]
    fn read_parses_last_block_without_trailing_blank_line() {
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

        let cur = Cursor::new(input.as_bytes());
        let mut parser = Parser::<YPBankTextRecord, _>::new(cur);
        let rec = parser.next().expect("Should have one record");

        assert!(parser.next().is_none());
        assert!(parser.read_error.is_none(), "Read error: {}", parser.read_error.unwrap());

        assert_eq!(rec.id, 3);
        assert_eq!(rec.transaction_type, TransactionType::Transfer);
        assert_eq!(rec.from_user_id, 1);
        assert_eq!(rec.to_user_id, 2);
        assert_eq!(rec.amount, 7);
        assert_eq!(rec.timestamp, 3);
        assert_eq!(rec.transaction_status, TransactionStatus::Success);
        assert_eq!(rec.description, "No trailing blank");
    }

    #[test]
    fn read_line_without_colon_errors() {
        let input = r#"
TX_ID 123
TX_TYPE: DEPOSIT
"#;

        let cur = Cursor::new(input.as_bytes());
        let mut parser = Parser::<YPBankTextRecord, _>::new(cur);
        assert!(parser.next().is_none());

        if let Some(err) = parser.read_error {
             assert!(matches!(err, TextRecordError::MissingColonAfterKey));
        } else {
             panic!("Expected read_error to be set");
        }
    }

    #[test]
    fn read_invalid_data_types_errors() {
        let input_negative_id = r#"
TX_ID: -5
TX_TYPE: DEPOSIT
"#;
        let cur = Cursor::new(input_negative_id.as_bytes());
        let mut parser = Parser::<YPBankTextRecord, _>::new(cur);
        assert!(parser.next().is_none());
        assert!(matches!(parser.read_error.unwrap(), TextRecordError::ParseError { .. }));

        let input_bad_amount = r#"
TX_ID: 10
AMOUNT: not_a_number
"#;
        let cur = Cursor::new(input_bad_amount.as_bytes());
        let mut parser = Parser::<YPBankTextRecord, _>::new(cur);
        assert!(parser.next().is_none());
        assert!(matches!(parser.read_error.unwrap(), TextRecordError::ParseError { .. }));

        let input_bad_status = r#"
TX_ID: 11
STATUS: UNKNOWN_STATUS
"#;
        let cur = Cursor::new(input_bad_status.as_bytes());
        let mut parser = Parser::<YPBankTextRecord, _>::new(cur);
        assert!(parser.next().is_none());
        assert!(matches!(parser.read_error.unwrap(), TextRecordError::ParseError { .. }));
    }

    #[test]
    fn read_errors_on_empty_source() {
        let input = "";
        let cur = Cursor::new(input.as_bytes());
        let mut parser = Parser::<YPBankTextRecord, _>::new(cur);
        let result = parser.next();

        assert!(result.is_none());
        assert!(parser.read_error.is_none(), "Expected no read error on empty source, got: {:?}", parser.read_error);
    }

    #[test]
    fn read_fails_on_extra_fields() {
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
"#;
        let cur = Cursor::new(input.as_bytes());
        let mut parser = Parser::<YPBankTextRecord, _>::new(cur);
        assert!(parser.next().is_none());
        assert!(matches!(parser.read_error.unwrap(), TextRecordError::ParseError { .. }));
    }
}