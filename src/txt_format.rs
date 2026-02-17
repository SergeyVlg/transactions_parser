pub mod txt_format {
    use std::collections::HashMap;
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

    //---------------
    //реализация через serde, плюс оптимизации по выделению памяти

    use serde::Deserialize;
    use std::fs::File;
    use std::io;

    #[derive(Debug, Deserialize)]
    pub struct Transaction {
        #[serde(rename = "TX_ID")]
        pub id: u64,
        #[serde(rename = "TX_TYPE")]
        pub transaction_type: String,
        #[serde(rename = "AMOUNT")]
        pub amount: f64,
    }

    pub struct FastTransactionLoader {
        reader: BufReader<File>,
        // Переиспользуем память вектора и строк
        kv_pairs: Vec<(String, String)>,
        line_buf: String,
    }

    impl FastTransactionLoader {
        pub fn new(path: &str) -> io::Result<Self> {
            Ok(Self {
                reader: BufReader::new(File::open(path)?),
                kv_pairs: Vec::with_capacity(8), // Заранее аллоцируем место под поля, лучше в именованную константу
                line_buf: String::with_capacity(128),
            })
        }

        fn parse_block(&self) -> Result<Transaction, String> {
            // Конвертируем Vec в Value для Serde
            let map: HashMap<_, _> = self.kv_pairs.iter().cloned().collect();
            serde_json::to_value(map) //вот тут не оч, т.к. приплетается конверсия в json. Лучше бы как-то самому это распознавать
                .and_then(serde_json::from_value)
                .map_err(|e| e.to_string())
        }
    }

    impl Iterator for FastTransactionLoader {
        type Item = Result<Transaction, String>;

        fn next(&mut self) -> Option<Self::Item> {
            self.kv_pairs.clear();

            while self.reader.read_line(&mut self.line_buf).unwrap_or(0) > 0 {
                let trimmed = self.line_buf.trim();

                if trimmed.is_empty() {
                    if !self.kv_pairs.is_empty() {
                        self.line_buf.clear();
                        return Some(self.parse_block());
                    }
                } else if let Some((k, v)) = trimmed.split_once(':') {
                    // Сохраняем значения, минимизируя аллокации
                    self.kv_pairs.push((k.trim().to_owned(), v.trim().to_owned()));
                }

                self.line_buf.clear(); // Очищаем буфер строки для следующей итерации
            }

            // Обработка последнего блока
            if !self.kv_pairs.is_empty() {
                let res = self.parse_block();
                self.kv_pairs.clear();
                Some(res)
            } else {
                None
            }
        }
    }

    //второй вариант, без json
    use serde::Deserialize;
    use std::collections::HashMap;
    use std::io::{BufRead, BufReader};
    use std::fs::File;

    // Используем этот крейт для автоматического парсинга строк в числа
    // Добавь в Cargo.toml: serde_with = "3.0"
    use serde_with::{serde_as, DisplayFromStr};

    #[serde_as]
    #[derive(Debug, Deserialize)]
    struct Transaction {
        #[serde(rename = "TX_ID")]
        #[serde_as(as = "DisplayFromStr")] // Магия: берет строку и делает parse()
        pub id: u64,

        #[serde(rename = "TX_TYPE")]
        pub transaction_type: String,

        #[serde(rename = "AMOUNT")]
        #[serde_as(as = "DisplayFromStr")] // Магия: берет строку и делает parse()
        pub amount: f64,
    }

    fn process_block(map: HashMap<String, String>) -> Result<Transaction, String> {
        Transaction::deserialize(serde::de::value::MapDeserializer::new(map.into_iter()))
            .map_err(|e: serde::de::value::Error| e.to_string())
    }
}