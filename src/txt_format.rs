use crate::common::{TransactionStatus, TransactionType};
use crate::{Readable, Writable};
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};

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

impl<R: Read> Readable<R, TextRecordError> for YPBankTextRecord {
    type Reader = BufReader<R>;

    fn build_reader(source: R) -> Self::Reader {
        BufReader::new(source)
    }

    fn read(reader: &mut Self::Reader) -> Result<YPBankTextRecord, TextRecordError> {
        let mut kv_pairs: HashMap<String, String> = HashMap::with_capacity(8); //сразу аллоцируем память
        let mut line_buf = String::with_capacity(128);

        if reader.fill_buf()
            .map_err(|e| TextRecordError::ReadLineError(e))?
            .is_empty() {
            return Err(TextRecordError::SourceIsEmpty)
        }

        loop {
            match reader.read_line(&mut line_buf) {
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

        if !kv_pairs.is_empty() {
            let res = Self::parse_transaction(&mut kv_pairs)?;
            kv_pairs.clear();

            return Ok(res);
        }

        Err(TextRecordError::EmptyLinesAtEndOfFile)
    }
}

impl Writable<std::io::Error> for YPBankTextRecord {
    fn write<W: Write>(&self, writer: &mut W) -> Result<(), std::io::Error> {
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
    fn parse_transaction(map: &mut HashMap<String, String>) -> Result<Self, serde::de::value::Error> {
        Self::deserialize(serde::de::value::MapDeserializer::new(map.drain()))
            .map_err(|e: serde::de::value::Error| e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;
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
    fn writes_all_required_fields_and_blank_separator_line() {
        let rec = sample_record();
        let mut out = Cursor::new(Vec::<u8>::new());

        rec.write(&mut out).unwrap();

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
        assert!(s.contains("DESCRIPTION: \"User transfer\"\n"));

        // запись должна заканчиваться пустой строкой-разделителем
        assert!(s.ends_with("\n\n"), "expected record to end with a blank line separator, got: {s:?}");
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
        let mut parser = Parser::<YPBankTextRecord, _, _>::new(cur);
        let rec = parser.next().expect("Should have a record").expect("Should parse successfully");

        assert_eq!(rec.id, 2312321321);
        assert_eq!(rec.transaction_type, TransactionType::Transfer);
        assert_eq!(rec.from_user_id, 123);
        assert_eq!(rec.to_user_id, 987);
        assert_eq!(rec.amount, 1000);
        assert_eq!(rec.timestamp, 1633056800000);
        assert_eq!(rec.transaction_status, TransactionStatus::Failure);
        assert_eq!(rec.description, "\"User transfer\"");

        assert!(parser.next().is_none(), "Should be consumed");
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
        let mut parser = Parser::<YPBankTextRecord, _, _>::new(cur);

        let r1 = parser.next().expect("Should have first record").expect("Should parse first record");
        let r2 = parser.next().expect("Should have second record").expect("Should parse second record");

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

        assert!(parser.next().is_none());
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
        let mut parser = Parser::<YPBankTextRecord, _, _>::new(cur);
        let rec = parser.next().expect("Should have one record").expect("Should parse");

        assert_eq!(rec.id, 3);
        assert_eq!(rec.transaction_type, TransactionType::Transfer);
        assert_eq!(rec.from_user_id, 1);
        assert_eq!(rec.to_user_id, 2);
        assert_eq!(rec.amount, 7);
        assert_eq!(rec.timestamp, 3);
        assert_eq!(rec.transaction_status, TransactionStatus::Success);
        assert_eq!(rec.description, "\"No trailing blank\"");
        assert!(parser.next().is_none());
    }

    #[test]
    fn read_line_without_colon_errors() {
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

        let cur = Cursor::new(input.as_bytes());
        let mut parser = Parser::<YPBankTextRecord, _, _>::new(cur);
        assert!(parser.next().is_none());
        assert!(matches!(parser.read_error.unwrap(), TextRecordError::MissingColonAfterKey));
    }

    #[test]
    fn read_invalid_data_types_errors() {

        let input_negative_id = r#"
TX_ID: -5
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 10
AMOUNT: 100
TIMESTAMP: 1
STATUS: SUCCESS
DESCRIPTION: "Negative ID"
"#;
        let cur = Cursor::new(input_negative_id.as_bytes());
        let mut parser = Parser::<YPBankTextRecord, _, _>::new(cur);
        assert!(parser.next().is_none());
        assert!(matches!(parser.read_error.unwrap(), TextRecordError::ParseError { .. }));

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
        let cur = Cursor::new(input_bad_amount.as_bytes());
        let mut parser = Parser::<YPBankTextRecord, _, _>::new(cur);
        assert!(parser.next().is_none());
        assert!(matches!(parser.read_error.unwrap(), TextRecordError::ParseError { .. }));

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
        let cur = Cursor::new(input_bad_status.as_bytes());
        let mut parser = Parser::<YPBankTextRecord, _, _>::new(cur);
        assert!(parser.next().is_none());
        assert!(matches!(parser.read_error.unwrap(), TextRecordError::ParseError { .. }));
    }

    #[test]
    fn read_errors_on_empty_source() {
        let input = "";
        let cur = Cursor::new(input.as_bytes());
        let mut parser = Parser::<YPBankTextRecord, _, _>::new(cur);
        let result = parser.next();

        assert!(result.is_none());
        assert!(matches!(parser.read_error.unwrap(), TextRecordError::SourceIsEmpty));
    }

    #[test]
    fn read_ignores_extra_fields() {
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
        let cur = Cursor::new(input.as_bytes());
        let mut parser = Parser::<YPBankTextRecord, _, _>::new(cur);
        assert!(parser.next().is_none());
        assert!(matches!(parser.read_error.unwrap(), TextRecordError::ParseError { .. }));
    }
}