use std::error::Error;
use std::path::PathBuf;

use tick_to_ohlcv::{
    aggregate_csv_files, discover_files, parse_interval_seconds, write_candles_csv,
    write_candles_parquet, CsvTradeMapping, OutputFormat, TimestampUnit,
};

#[derive(Debug)]
struct CsvArgs {
    paths: Vec<PathBuf>,
    input_root: Option<PathBuf>,
    include: Vec<String>,
    exclude: Vec<String>,
    timestamp_column: Option<String>,
    timestamp_unit: TimestampUnit,
    price_column: Option<String>,
    size_column: Option<String>,
    symbol: Option<String>,
    symbol_column: Option<String>,
    turnover_column: Option<String>,
    side_column: Option<String>,
    trade_id_column: Option<String>,
    delimiter: u8,
    interval: String,
    interval_seconds: Option<i64>,
    fill_gaps: bool,
    output_format: Option<OutputFormat>,
    output: Option<PathBuf>,
}

impl Default for CsvArgs {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            input_root: None,
            include: Vec::new(),
            exclude: Vec::new(),
            timestamp_column: None,
            timestamp_unit: TimestampUnit::Milliseconds,
            price_column: None,
            size_column: None,
            symbol: None,
            symbol_column: None,
            turnover_column: None,
            side_column: None,
            trade_id_column: None,
            delimiter: b',',
            interval: "1m".to_string(),
            interval_seconds: None,
            fill_gaps: false,
            output_format: None,
            output: None,
        }
    }
}

fn main() {
    if let Err(exc) = run() {
        eprintln!("error: {exc}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("csv") => run_csv(parse_csv_args(args.collect())?),
        Some("-h") | Some("--help") | None => {
            print_help();
            Ok(())
        }
        Some(other) => Err(format!("unsupported source: {other}").into()),
    }
}

fn run_csv(args: CsvArgs) -> Result<(), Box<dyn Error>> {
    if args.symbol.is_none() && args.symbol_column.is_none() {
        return Err("CSV input requires --symbol or --symbol-column".into());
    }
    let timestamp_column = required(args.timestamp_column, "--timestamp-column")?;
    let price_column = required(args.price_column, "--price-column")?;
    let size_column = required(args.size_column, "--size-column")?;
    let paths = discover_files(
        &args.paths,
        args.input_root.as_deref(),
        &args.include,
        &args.exclude,
    )?;
    if paths.is_empty() {
        return Err("CSV input requires at least one input file".into());
    }

    let interval_seconds = match args.interval_seconds {
        Some(value) if value > 0 => value,
        Some(_) => return Err("--interval-seconds must be positive".into()),
        None => parse_interval_seconds(&args.interval)?,
    };
    let mapping = CsvTradeMapping {
        timestamp_column,
        price_column,
        size_column,
        symbol: args.symbol,
        symbol_column: args.symbol_column,
        turnover_column: args.turnover_column,
        side_column: args.side_column,
        trade_id_column: args.trade_id_column,
        timestamp_unit: args.timestamp_unit,
    };
    let candles = aggregate_csv_files(
        &paths,
        &mapping,
        interval_seconds,
        args.fill_gaps,
        args.delimiter,
    )?;
    match infer_output_format(args.output.as_ref(), args.output_format) {
        OutputFormat::Csv => write_candles_csv(&candles, args.output.as_deref())?,
        OutputFormat::Parquet => {
            let output = args
                .output
                .as_deref()
                .ok_or("Parquet output requires --output")?;
            write_candles_parquet(&candles, output)?;
        }
    }
    Ok(())
}

fn parse_csv_args(values: Vec<String>) -> Result<CsvArgs, Box<dyn Error>> {
    let mut parsed = CsvArgs::default();
    let mut index = 0;
    while index < values.len() {
        let item = &values[index];
        match item.as_str() {
            "--input-root" => {
                parsed.input_root = Some(PathBuf::from(take_value(&values, &mut index, item)?))
            }
            "--include" => parsed.include.push(take_value(&values, &mut index, item)?),
            "--exclude" => parsed.exclude.push(take_value(&values, &mut index, item)?),
            "--timestamp-column" => {
                parsed.timestamp_column = Some(take_value(&values, &mut index, item)?)
            }
            "--timestamp-unit" => {
                parsed.timestamp_unit = match take_value(&values, &mut index, item)?.as_str() {
                    "ms" => TimestampUnit::Milliseconds,
                    "s" => TimestampUnit::Seconds,
                    other => return Err(format!("unsupported timestamp unit: {other}").into()),
                }
            }
            "--price-column" => parsed.price_column = Some(take_value(&values, &mut index, item)?),
            "--size-column" => parsed.size_column = Some(take_value(&values, &mut index, item)?),
            "--symbol" => parsed.symbol = Some(take_value(&values, &mut index, item)?),
            "--symbol-column" => {
                parsed.symbol_column = Some(take_value(&values, &mut index, item)?)
            }
            "--turnover-column" => {
                parsed.turnover_column = Some(take_value(&values, &mut index, item)?)
            }
            "--side-column" => parsed.side_column = Some(take_value(&values, &mut index, item)?),
            "--trade-id-column" => {
                parsed.trade_id_column = Some(take_value(&values, &mut index, item)?)
            }
            "--delimiter" => {
                let value = take_value(&values, &mut index, item)?;
                parsed.delimiter = value
                    .as_bytes()
                    .first()
                    .copied()
                    .ok_or("delimiter must not be empty")?;
            }
            "--interval" => parsed.interval = take_value(&values, &mut index, item)?,
            "--interval-seconds" => {
                parsed.interval_seconds = Some(take_value(&values, &mut index, item)?.parse()?);
            }
            "--fill-gaps" => parsed.fill_gaps = true,
            "--output-format" => {
                parsed.output_format =
                    Some(match take_value(&values, &mut index, item)?.as_str() {
                        "csv" => OutputFormat::Csv,
                        "parquet" => OutputFormat::Parquet,
                        other => return Err(format!("unsupported output format: {other}").into()),
                    });
            }
            "--output" => {
                parsed.output = Some(PathBuf::from(take_value(&values, &mut index, item)?))
            }
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            _ if item.starts_with("--") => return Err(format!("unknown option: {item}").into()),
            _ => parsed.paths.push(PathBuf::from(item)),
        }
        index += 1;
    }
    Ok(parsed)
}

fn take_value(
    values: &[String],
    index: &mut usize,
    option: &str,
) -> Result<String, Box<dyn Error>> {
    *index += 1;
    values
        .get(*index)
        .cloned()
        .ok_or_else(|| format!("{option} requires a value").into())
}

fn required(value: Option<String>, option: &str) -> Result<String, Box<dyn Error>> {
    value.ok_or_else(|| format!("{option} is required").into())
}

fn infer_output_format(output: Option<&PathBuf>, explicit: Option<OutputFormat>) -> OutputFormat {
    if let Some(format) = explicit {
        return format;
    }
    if output
        .and_then(|path| path.extension())
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("parquet") || ext.eq_ignore_ascii_case("pq"))
    {
        return OutputFormat::Parquet;
    }
    OutputFormat::Csv
}

fn print_help() {
    println!(
        "tick-to-ohlcv csv [paths...] \\
  --input-root DIR --include PATTERN --exclude PATTERN \\
  --symbol SYMBOL | --symbol-column COLUMN \\
  --timestamp-column COLUMN --price-column COLUMN --size-column COLUMN \\
  [--turnover-column COLUMN] [--side-column COLUMN] [--trade-id-column COLUMN] \\
  [--timestamp-unit ms|s] [--delimiter ,] [--interval 1m] [--fill-gaps] \\
  [--output-format csv|parquet] [--output PATH]"
    );
}
