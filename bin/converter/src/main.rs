use clap::Parser as ClapParser;
use std::fs::File;
use std::io::{Error, ErrorKind, Read, Write};
use std::path::Path;
use transactions_parser::{Parser, Readable, Writable, YPBankBinRecord, YPBankCsvRecord, YPBankTextRecord};

#[derive(ClapParser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    input: String,

    #[arg(long)]
    input_format: String,

    #[arg(long)]
    output_format: String
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    Path::new(&args.input).try_exists()?;

    if args.input_format == args.output_format {
        return Err(Error::new(ErrorKind::InvalidInput, "Input and output formats are the same"));
    }

    let input_file = File::open(args.input)?;
    let output = std::io::stdout();

    match (args.input_format.as_str(), args.output_format.as_str()) {
        ("txt", "csv") => convert::<YPBankTextRecord, YPBankCsvRecord, _, _>(input_file, output),
        ("txt", "bin") => convert::<YPBankTextRecord, YPBankBinRecord, _, _>(input_file, output),
        ("csv", "txt") => convert::<YPBankCsvRecord, YPBankTextRecord, _, _>(input_file, output),
        ("csv", "bin") => convert::<YPBankCsvRecord, YPBankBinRecord, _, _>(input_file, output),
        ("bin", "txt") => convert::<YPBankBinRecord, YPBankTextRecord, _, _>(input_file, output),
        ("bin", "csv") => convert::<YPBankBinRecord, YPBankCsvRecord, _, _>(input_file, output),

        _ => {
            Err(Error::new(ErrorKind::InvalidInput, format!("Unsupported format combination: {} -> {}", args.input_format, args.output_format)))
        }
    }
}

fn convert<TFrom, TTo, TSource, TTarget>(source: TSource, target: TTarget) -> Result<(), Error>
where
    TFrom: Readable<TSource>,
    TTo: Writable,
    TSource: Read,
    TTarget: Write
{
    let mut parser = Parser::<TFrom, _>::new(source);
    let mut serializer = transactions_parser::Serializer::<TTo, _>::new(target);

    let target_records = parser
        .by_ref()
        .map(|res| TTo::from(res.into()));
    
    serializer.serialize(target_records).map_err(|e| e.into())?;
    
    if let Some(err) = parser.read_error {
        return Err(err.into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn get_csv_input() -> String {
        "TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION\n\
         1,DEPOSIT,0,10,100,1600000000,SUCCESS,\"Test deposit\"\n".to_string()
    }

    fn get_text_expected() -> String {
        "TX_ID: 1\n\
         TX_TYPE: DEPOSIT\n\
         FROM_USER_ID: 0\n\
         TO_USER_ID: 10\n\
         AMOUNT: 100\n\
         TIMESTAMP: 1600000000\n\
         STATUS: SUCCESS\n\
         DESCRIPTION: \"Test deposit\"\n\n".to_string()
    }

    #[test]
    fn test_csv_to_text_conversion() {
        let input_data = get_csv_input();
        let input_cursor = Cursor::new(input_data.as_bytes());
        let mut output_cursor = Cursor::new(Vec::new());

        let result = convert::<YPBankCsvRecord, YPBankTextRecord, _, _>(
            input_cursor,
            &mut output_cursor,
        );

        assert!(result.is_ok());
        let output_bytes = output_cursor.into_inner();
        let output_str = String::from_utf8(output_bytes).expect("Output should be valid UTF-8");

        assert_eq!(output_str, get_text_expected());
    }

    #[test]
    fn test_text_to_csv_conversion() {
        let input_data = get_text_expected();
        let input_cursor = Cursor::new(input_data.as_bytes());
        let mut output_cursor = Cursor::new(Vec::new());

        convert::<YPBankTextRecord, YPBankCsvRecord, _, _>(
            input_cursor,
            &mut output_cursor,
        ).expect("Conversion failed");

        let output_bytes = output_cursor.into_inner();
        let output_str = String::from_utf8(output_bytes).expect("Output should be valid UTF-8");

        assert_eq!(output_str, get_csv_input());
    }

    #[test]
    fn test_text_to_bin_and_back_to_text() {
        let original_text = get_text_expected();
        let input_cursor = Cursor::new(original_text.as_bytes());
        let mut bin_output = Cursor::new(Vec::new());

        convert::<YPBankTextRecord, YPBankBinRecord, _, _>(
            input_cursor,
            &mut bin_output,
        ).expect("Text to Bin failed");

        let bin_data = bin_output.into_inner();
        assert!(!bin_data.is_empty());
        assert_eq!(&bin_data[0..4], b"YPBN"); // Проверка magic bytes

        let bin_input = Cursor::new(bin_data);
        let mut text_output = Cursor::new(Vec::new());

        convert::<YPBankBinRecord, YPBankTextRecord, _, _>(
            bin_input,
            &mut text_output,
        ).expect("Bin to Text failed");

        let final_text = String::from_utf8(text_output.into_inner()).unwrap();
        assert_eq!(final_text, original_text);
    }

    #[test]
    fn test_csv_to_bin_and_back_to_csv() {
        let original_csv = get_csv_input();
        let input_cursor = Cursor::new(original_csv.as_bytes());
        let mut bin_output = Cursor::new(Vec::new());

        convert::<YPBankCsvRecord, YPBankBinRecord, _, _>(
            input_cursor,
            &mut bin_output,
        ).expect("CSV to Bin failed");

        let bin_data = bin_output.into_inner();
        let bin_input = Cursor::new(bin_data);
        let mut csv_output = Cursor::new(Vec::new());

        convert::<YPBankBinRecord, YPBankCsvRecord, _, _>(
            bin_input,
            &mut csv_output,
        ).expect("Bin to CSV failed");

        let final_csv = String::from_utf8(csv_output.into_inner()).unwrap();
        assert_eq!(final_csv, original_csv);
    }

    #[test]
    fn test_conversion_fails_on_invalid_input() {
        let invalid_data = "NOT,A,VALID,CSV\n1,2,3";
        let input_cursor = Cursor::new(invalid_data.as_bytes());
        let mut output_cursor = Cursor::new(Vec::new());

        let result = convert::<YPBankCsvRecord, YPBankTextRecord, _, _>(
            input_cursor,
            &mut output_cursor,
        );

        assert!(result.is_err());
    }
}
