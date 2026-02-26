use clap::Parser as ClapParser;
use std::collections::HashSet;
use std::fs::File;
use std::io::{Error, ErrorKind, Read, Write};
use std::path::Path;
use transactions_parser::{Parser, Readable, Transaction, YPBankBinRecord, YPBankCsvRecord, YPBankTextRecord};

#[derive(ClapParser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long = "file1")]
    first_file: String,

    #[arg(long = "file1-format")]
    first_file_format: String,

    #[arg(long = "file2")]
    second_file: String,

    #[arg(long = "file2-format")]
    second_file_format: String,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    Path::new(&args.first_file).try_exists().map_err(|e| format!("First file path wrong, error: {}", e)).expect("First file path wrong");
    Path::new(&args.second_file).try_exists().map_err(|e| format!("Second file path wrong, error: {}", e)).expect("Second file path wrong");

    let first_file = File::open(args.first_file).map_err(|e| format!("Open first file error: {}", e)).expect("First file does not exist");
    let second_file = File::open(args.second_file).map_err(|e| format!("Open second file error: {}", e)).expect("Second file does not exist");

    match (args.first_file_format.as_str(), args.second_file_format.as_str()) {
        ("txt", "csv") => compare::<YPBankTextRecord, YPBankCsvRecord, _, _, _>(first_file, second_file, std::io::stdout()),
        ("txt", "bin") => compare::<YPBankTextRecord, YPBankBinRecord, _, _, _>(first_file, second_file, std::io::stdout()),
        ("csv", "txt") => compare::<YPBankCsvRecord, YPBankTextRecord, _, _, _>(first_file, second_file, std::io::stdout()),
        ("csv", "bin") => compare::<YPBankCsvRecord, YPBankBinRecord, _, _, _>(first_file, second_file, std::io::stdout()),
        ("bin", "txt") => compare::<YPBankBinRecord, YPBankTextRecord, _, _, _>(first_file, second_file, std::io::stdout()),
        ("bin", "csv") => compare::<YPBankBinRecord, YPBankCsvRecord, _, _, _>(first_file, second_file, std::io::stdout()),

        _ => {
            Err(Error::new(ErrorKind::InvalidInput, format!("Unsupported format combination: {} -> {}", args.first_file_format, args.second_file_format)))
        }
    }
}

fn compare<TFormat1, TFormat2, TSource1, TSource2, TOutput>(first_source: TSource1, second_source: TSource2, mut output: TOutput) -> Result<(), Error>
where
    TFormat1: Readable<TSource1>,
    TFormat2: Readable<TSource2>,
    TSource1: Read,
    TSource2: Read,
    TOutput: Write
{
    let mut first_parser = Parser::<TFormat1, _>::new(first_source);
    let mut second_parser = Parser::<TFormat2, _>::new(second_source);

    let mut first_set: HashSet<Transaction> = first_parser
        .by_ref()
        .map(|res| res.into())
        .collect();

    if let Some(err) = first_parser.read_error {
        return Err(err.into());
    }

    let mut files_is_same = true;

    second_parser.by_ref()
    .map(|res| res.into())
    .try_for_each(|transaction: Transaction| -> Result<(), Error> {
         if !first_set.remove(&transaction) {
             files_is_same = false;
             writeln!(output, "Transaction with id {} is only in file 2", transaction.id)?;
         }
        Ok(())
     })?;

    if let Some(err) = second_parser.read_error {
        return Err(err.into());
    }

    files_is_same &= first_set.is_empty();

    if files_is_same {
        writeln!(output, "Files are identical")?;

        return Ok(());
    }

    for transaction in first_set {
        writeln!(output, "Transaction with id {} is only in file 1", transaction.id)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use transactions_parser::{YPBankBinRecord, YPBankCsvRecord, YPBankTextRecord, Transaction, TransactionType, TransactionStatus, Writable};

    fn create_bin_data(transactions: Vec<Transaction>) -> Vec<u8> {
        let mut buffer = Vec::new();
        for tx in transactions {
            let record: YPBankBinRecord = tx.into();
            record.write(&mut buffer).unwrap();
        }
        buffer
    }

    fn create_transaction(id: u64) -> Transaction {
        Transaction {
            id,
            transaction_type: TransactionType::Deposit,
            from_user_id: 1,
            to_user_id: 2,
            amount: 100,
            timestamp: 1234567890,
            transaction_status: TransactionStatus::Success,
            description: "test".to_string(),
        }
    }

    #[test]
    fn test_csv_vs_csv_identical() {
        let csv_data = "\
TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
1,DEPOSIT,1,2,100,1234567890,SUCCESS,\"test\"
2,DEPOSIT,1,2,100,1234567890,SUCCESS,\"test\"
3,DEPOSIT,1,2,100,1234567890,SUCCESS,\"test\"
";
        let source1 = Cursor::new(csv_data);
        let source2 = Cursor::new(csv_data);
        let mut output = Vec::new();

        let result = compare::<YPBankCsvRecord, YPBankCsvRecord, _, _, _>(source1, source2, &mut output);

        assert!(result.is_ok());
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("Files are identical"));
    }

    #[test]
    fn test_txt_vs_csv_different() {
        let txt_data = "\
TX_ID: 1
TX_TYPE: DEPOSIT
FROM_USER_ID: 1
TO_USER_ID: 2
AMOUNT: 100
TIMESTAMP: 1234567890
STATUS: SUCCESS
DESCRIPTION: \"test\"

TX_ID: 2
TX_TYPE: DEPOSIT
FROM_USER_ID: 1
TO_USER_ID: 2
AMOUNT: 100
TIMESTAMP: 1234567890
STATUS: SUCCESS
DESCRIPTION: \"test\"

TX_ID: 4
TX_TYPE: DEPOSIT
FROM_USER_ID: 1
TO_USER_ID: 2
AMOUNT: 100
TIMESTAMP: 1234567890
STATUS: SUCCESS
DESCRIPTION: \"test\"
";

        let csv_data = "\
TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
2,DEPOSIT,1,2,100,1234567890,SUCCESS,\"test\"
3,DEPOSIT,1,2,100,1234567890,SUCCESS,\"test\"
4,DEPOSIT,1,2,100,1234567890,SUCCESS,\"test\"
";

        let source1 = Cursor::new(txt_data);
        let source2 = Cursor::new(csv_data);
        let mut output = Vec::new();

        let result = compare::<YPBankTextRecord, YPBankCsvRecord, _, _, _>(source1, source2, &mut output);

        assert!(result.is_ok());
        let output_str = String::from_utf8(output).unwrap();

        assert!(!output_str.contains("Files are identical"));
        assert!(output_str.contains("Transaction with id 1 is only in file 1"));
        assert!(output_str.contains("Transaction with id 3 is only in file 2"));

        assert!(!output_str.contains("Transaction with id 2"));
        assert!(!output_str.contains("Transaction with id 4"));
    }

    #[test]
    fn test_bin_vs_csv_different() {
        let bin_data = create_bin_data(vec![
            create_transaction(10),
            create_transaction(20),
            create_transaction(30)
        ]);

        let csv_data = "\
TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
10,DEPOSIT,1,2,100,1234567890,SUCCESS,\"test\"
30,DEPOSIT,1,2,100,1234567890,SUCCESS,\"test\"
40,DEPOSIT,1,2,100,1234567890,SUCCESS,\"test\"
";

        let source1 = Cursor::new(bin_data);
        let source2 = Cursor::new(csv_data);
        let mut output = Vec::new();

        let result = compare::<YPBankBinRecord, YPBankCsvRecord, _, _, _>(source1, source2, &mut output);

        assert!(result.is_ok());
        let output_str = String::from_utf8(output).unwrap();

        assert!(output_str.contains("Transaction with id 20 is only in file 1"));
        assert!(output_str.contains("Transaction with id 40 is only in file 2"));

        assert!(!output_str.contains("Transaction with id 10"));
        assert!(!output_str.contains("Transaction with id 30"));
    }

    #[test]
    fn test_txt_vs_bin_different() {
        let txt_data = "\
TX_ID: 1
TX_TYPE: DEPOSIT
FROM_USER_ID: 1
TO_USER_ID: 2
AMOUNT: 100
TIMESTAMP: 1234567890
STATUS: SUCCESS
DESCRIPTION: \"test\"

TX_ID: 5
TX_TYPE: DEPOSIT
FROM_USER_ID: 1
TO_USER_ID: 2
AMOUNT: 100
TIMESTAMP: 1234567890
STATUS: SUCCESS
DESCRIPTION: \"test\"

TX_ID: 6
TX_TYPE: DEPOSIT
FROM_USER_ID: 1
TO_USER_ID: 2
AMOUNT: 100
TIMESTAMP: 1234567890
STATUS: SUCCESS
DESCRIPTION: \"test\"
";

        let bin_data = create_bin_data(vec![
            create_transaction(1),
            create_transaction(2),
            create_transaction(6)
        ]);

        let source1 = Cursor::new(txt_data);
        let source2 = Cursor::new(bin_data);
        let mut output = Vec::new();

        let result = compare::<YPBankTextRecord, YPBankBinRecord, _, _, _>(source1, source2, &mut output);

        assert!(result.is_ok());
        let output_str = String::from_utf8(output).unwrap();

        assert!(!output_str.contains("Files are identical"));
        assert!(output_str.contains("Transaction with id 5 is only in file 1"));
        assert!(output_str.contains("Transaction with id 2 is only in file 2"));

        assert!(!output_str.contains("Transaction with id 1"));
        assert!(!output_str.contains("Transaction with id 6"));
    }
}
