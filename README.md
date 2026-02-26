# transactions_parser

Библиотека для парсинга и сериализации банковских транзакций в различных форматах.

Этот крейт предоставляет унифицированный интерфейс для чтения (`Parser`) и записи (`Serializer`) данных о транзакциях, абстрагируясь от конкретного формата хранения.

## Поддерживаемые форматы

Библиотека поддерживает работу со следующими форматами через реализацию трейтов `Readable` и `Writable`:

*   **CSV** (`YPBankCsvRecord`): Текстовый формат с разделением запятыми.
*   **Binary** (`YPBankBinRecord`): Специализированный бинарный формат (с магическими байтами `YPBN`).
*   **Custom Text** (`YPBankTextRecord`): Текстовый формат в виде пар "ключ-значение".

## Пример использования

### Чтение (Парсинг)

Для чтения данных используется универсальный `Parser`. Необходимо указать тип записи (например, `YPBankCsvRecord`), в который будут парситься данные, а затем преобразовывать их в общий тип `Transaction`.

```rust
use std::fs::File;
use transactions_parser::{Parser, YPBankCsvRecord, Transaction};

fn main() -> std::io::Result<()> {
    let file = File::open("data.csv")?;
    // Создаем парсер для CSV формата
    let parser = Parser::<YPBankCsvRecord, _>::new(file);

    for record in parser {
        let transaction: Transaction = record.into();
        println!("Обработана транзакция: ID={}", transaction.id);
    }
    
    Ok(())
}
```

### Запись (Сериализация)

Для записи используется `Serializer`. Он принимает итератор записей определенного формата.

```rust
use std::fs::File;
use transactions_parser::{Serializer, YPBankBinRecord, Transaction};

fn main() -> std::io::Result<()> {
    let file = File::create("data.bin")?;
    let mut serializer = Serializer::<YPBankBinRecord, _>::new(file);

    let transactions: Vec<Transaction> = vec![
        // ... список транзакций
    ];

    // Преобразуем транзакции в целевой формат записи перед сериализацией
    let records = transactions.into_iter().map(YPBankBinRecord::from);
    
    serializer.serialize(records)?;
    
    Ok(())
}
```
