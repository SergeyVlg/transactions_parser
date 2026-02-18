pub mod txt_format {
    use std::collections::HashMap;
    use crate::Parser;
    use std::error::Error;
    use std::fmt::{Display, Formatter};
    use std::io::{BufRead, BufReader, Read, Write};
    use std::str::FromStr;

    #[derive(Debug, Deserialize)]
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

    #[derive(Debug, Deserialize)]
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

    #[serde_as]
    #[derive(Debug, Deserialize)]
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
        fn from_read<R: Read>(reader: &mut R) -> Result<YPBankTextRecord, TextRecordError> {
            let mut buff_reader = BufReader::new(reader);
            let mut kv_pairs: HashMap<String, String> = HashMap::with_capacity(8); //сразу аллоцируем память
            let mut line_buf = String::with_capacity(128);

            while buff_reader.read_line(&mut line_buf).unwrap_or(0) > 0 { //вот тут может сожраться ошибка
                let trimmed_line = line_buf.trim();

                if trimmed_line.starts_with('#') {
                    continue;
                }

                if trimmed_line.is_empty() {
                    if !kv_pairs.is_empty() {
                        line_buf.clear();

                        return Ok(Self::parse_transaction(&mut kv_pairs)?);
                    }
                } else if let Some((k, v)) = trimmed_line.split_once(':') {
                    kv_pairs.insert(k.trim().to_owned(), v.trim().to_owned());
                }

                line_buf.clear()
            }

            // Обработка последнего блока
            if !kv_pairs.is_empty() {
                let res = Self::parse_transaction(&mut kv_pairs)?;
                kv_pairs.clear();

                return Ok(res);
            }

            Err(TextRecordError::ExcessFields) //тут возможно нужна более точная ошибка - по сути это если были пустые строки или комментарии в конце файла
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

        fn parse_transaction(map: &mut HashMap<String, String>) -> Result<Self, String> {
            Self::deserialize(MapDeserializer::new(map.drain()))
                .map_err(|e: serde::de::value::Error| e.to_string())
        }
    }

    //---------------
    //реализация через serde, плюс оптимизации по выделению памяти

    /*use serde::Deserialize;
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
    }*/

    //второй вариант, без json
    use serde::Deserialize;
    //use std::collections::HashMap;
    //use std::io::{BufRead, BufReader};
    use std::fs::File;
    use serde::de::value::MapDeserializer;
    // Используем этот крейт для автоматического парсинга строк в числа
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
        Transaction::deserialize(MapDeserializer::new(map.into_iter()))
            .map_err(|e: serde::de::value::Error| e.to_string())
    }
}