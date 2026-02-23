use std::fs::File;
use std::io::{Error, Read, Write};
use std::path::Path;
use clap::Parser as ClapParser;
use transactions_parser::{Parser, Readable, Writable, YPBankCsvRecord, YPBankBinRecord};

#[derive(ClapParser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input: String,

    #[arg(short, long)]
    input_format: String,

    #[arg(short, long)]
    output_format: String,
    output: String,
}

fn main() {
    let args = Args::parse();

    //По сути, надо считать все транзакции из исхдного файла, преобразовать их в нужный формат и записать в выходной файл
    //Возможно, стоит сделать это в виде конвейера, чтобы не держать все транзакции в памяти, а обрабатывать их по одной
    Path::new(&args.input).try_exists().expect("Invalid input path");

    if args.input_format == args.output_format {
        eprintln!("Input and output formats are the same");
        return;
    }

    if args.output.is_empty() {
        eprintln!("Output path is empty");
        return;
    }

    let input_file = File::open(args.input).unwrap();
    let output_file = File::create(args.output).unwrap();
    let result = convert_and_save::<_, YPBankCsvRecord, YPBankBinRecord, _>(input_file, output_file);//Надо сделать общий тип Transaction, и чтоб из него и в него все остальные типа могли конвертироваться

    //let parser = Parser::new() ;

    /*match args.input_format.as_str() {
        "txt" =>
    }*/
}

fn convert_and_save<TSource, TFrom, TTo, TTarget>(source: TSource, target: TTarget) -> Result<(), Error>
where
    TSource: Read,
    TFrom: Readable<TSource>,
    TFrom::Error: Into<Error>,
    TTo: Writable,
    TTarget: Write
{
    let mut parser = Parser::<TFrom, _>::new(source);
    let mut serializer = transactions_parser::Serializer::<TTo, _>::new(target);

    // Создаем "ленивый" итератор    
    let pipeline_iterator = parser
        .by_ref() // Берем по ссылке, чтобы parser остался доступен после цикла
        .filter_map(|res| Some(TTo::from(res))); // Конвертируем TFrom в TTo, пропуская неудачные конверсии
    
    serializer.serialize(pipeline_iterator).map_err(|e| e.into())?;

    
    if let Some(err) = parser.read_error {
        return Err(err.into());
    }

    Ok(())
}
