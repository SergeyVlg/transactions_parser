use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::ErrorKind;
use crate::IsEofError;

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
        std::io::Error::new(ErrorKind::InvalidInput, value)
    }
}

impl IsEofError for std::io::Error {
    fn is_eof(&self) -> bool {
        matches!(self.kind(), ErrorKind::UnexpectedEof)
    }
}