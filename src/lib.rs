use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use arrow_array::{ArrayRef, Float64Array, Int64Array, RecordBatch, StringArray, UInt64Array};
use arrow_schema::{DataType, Field, Schema};
use parquet::arrow::arrow_writer::ArrowWriter;
use parquet::file::properties::WriterProperties;

#[derive(Clone, Debug, PartialEq)]
pub struct Trade {
    pub symbol: String,
    pub ts_ms: i64,
    pub price: f64,
    pub size: f64,
    pub turnover: Option<f64>,
    pub side: Option<String>,
    pub trade_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Candle {
    pub symbol: String,
    pub ts: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub turnover: f64,
    pub trade_count: u64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub first_trade_ts_ms: Option<i64>,
    pub last_trade_ts_ms: Option<i64>,
}

impl Candle {
    pub fn from_trade(trade: &Trade, interval_seconds: i64) -> Self {
        let ts = interval_start_s(trade.ts_ms, interval_seconds);
        let turnover = trade.turnover.unwrap_or(trade.price * trade.size);
        let mut candle = Self {
            symbol: trade.symbol.to_uppercase(),
            ts,
            open: trade.price,
            high: trade.price,
            low: trade.price,
            close: trade.price,
            volume: trade.size,
            turnover,
            trade_count: 1,
            buy_volume: 0.0,
            sell_volume: 0.0,
            first_trade_ts_ms: Some(trade.ts_ms),
            last_trade_ts_ms: Some(trade.ts_ms),
        };
        candle.add_side_volume(trade);
        candle
    }

    pub fn flat(symbol: &str, ts: i64, close: f64) -> Self {
        Self {
            symbol: symbol.to_string(),
            ts,
            open: close,
            high: close,
            low: close,
            close,
            volume: 0.0,
            turnover: 0.0,
            trade_count: 0,
            buy_volume: 0.0,
            sell_volume: 0.0,
            first_trade_ts_ms: None,
            last_trade_ts_ms: None,
        }
    }

    pub fn datetime(&self) -> String {
        format_unix_seconds(self.ts)
    }

    pub fn add_trade(&mut self, trade: &Trade) {
        self.high = self.high.max(trade.price);
        self.low = self.low.min(trade.price);
        self.close = trade.price;
        self.volume += trade.size;
        self.turnover += trade.turnover.unwrap_or(trade.price * trade.size);
        self.trade_count += 1;
        self.last_trade_ts_ms = Some(trade.ts_ms);
        self.add_side_volume(trade);
    }

    fn add_side_volume(&mut self, trade: &Trade) {
        match trade
            .side
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str()
        {
            "buy" => self.buy_volume += trade.size,
            "sell" => self.sell_volume += trade.size,
            _ => {}
        }
    }
}

#[derive(Clone, Debug)]
pub struct CsvTradeMapping {
    pub timestamp_column: String,
    pub price_column: String,
    pub size_column: String,
    pub symbol: Option<String>,
    pub symbol_column: Option<String>,
    pub turnover_column: Option<String>,
    pub side_column: Option<String>,
    pub trade_id_column: Option<String>,
    pub timestamp_unit: TimestampUnit,
}

#[derive(Clone, Copy, Debug)]
pub enum TimestampUnit {
    Milliseconds,
    Seconds,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Csv,
    Parquet,
}

pub fn parse_interval_seconds(value: &str) -> Result<i64, Box<dyn Error>> {
    let text = value.trim().to_ascii_lowercase();
    if text.is_empty() {
        return Err("interval must not be empty".into());
    }
    if text.chars().all(|ch| ch.is_ascii_digit()) {
        let seconds: i64 = text.parse()?;
        if seconds <= 0 {
            return Err("interval must be positive".into());
        }
        return Ok(seconds);
    }

    let split = text
        .find(|ch: char| !ch.is_ascii_digit())
        .ok_or("interval must include a unit")?;
    if split == 0 {
        return Err(format!("interval must start with a number: {value}").into());
    }
    let amount: i64 = text[..split].parse()?;
    if amount <= 0 {
        return Err("interval must be positive".into());
    }
    let multiplier = match &text[split..] {
        "s" | "sec" | "second" | "seconds" => 1,
        "m" | "min" | "minute" | "minutes" => 60,
        "h" | "hr" | "hour" | "hours" => 60 * 60,
        "d" | "day" | "days" => 24 * 60 * 60,
        unit => return Err(format!("unsupported interval unit: {unit}").into()),
    };
    Ok(amount * multiplier)
}

pub fn interval_start_s(ts_ms: i64, interval_seconds: i64) -> i64 {
    let ts_s = ts_ms.div_euclid(1000);
    ts_s.div_euclid(interval_seconds) * interval_seconds
}

pub fn aggregate_trades(
    trades: impl IntoIterator<Item = Trade>,
    interval_seconds: i64,
    fill_gaps: bool,
) -> Vec<Candle> {
    let mut ordered: Vec<Trade> = trades.into_iter().collect();
    ordered.sort_by(|left, right| {
        (left.ts_ms, trade_id_sort_key(left)).cmp(&(right.ts_ms, trade_id_sort_key(right)))
    });

    let mut grouped: BTreeMap<(String, i64), Candle> = BTreeMap::new();
    for trade in ordered {
        if trade.price <= 0.0 || trade.size < 0.0 {
            continue;
        }
        let symbol = trade.symbol.to_uppercase();
        let ts = interval_start_s(trade.ts_ms, interval_seconds);
        grouped
            .entry((symbol, ts))
            .and_modify(|candle| candle.add_trade(&trade))
            .or_insert_with(|| Candle::from_trade(&trade, interval_seconds));
    }

    if !fill_gaps {
        return grouped.into_values().collect();
    }

    let mut out = Vec::new();
    let mut previous_by_symbol: HashMap<String, Candle> = HashMap::new();
    for ((symbol, _), current) in grouped {
        if let Some(previous) = previous_by_symbol.get(&symbol) {
            let mut gap_ts = previous.ts + interval_seconds;
            while gap_ts < current.ts {
                out.push(Candle::flat(&symbol, gap_ts, previous.close));
                gap_ts += interval_seconds;
            }
        }
        previous_by_symbol.insert(symbol, current.clone());
        out.push(current);
    }
    out
}

pub fn discover_files(
    explicit_paths: &[PathBuf],
    input_root: Option<&Path>,
    include_patterns: &[String],
    exclude_patterns: &[String],
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut seen = BTreeSet::new();
    let mut selected = Vec::new();
    for path in explicit_paths {
        add_file(path, &mut seen, &mut selected)?;
    }

    if let Some(root) = input_root {
        let includes = if include_patterns.is_empty() {
            vec!["**/*.csv".to_string()]
        } else {
            include_patterns.to_vec()
        };
        for path in walk_files(root)? {
            let relative = path.strip_prefix(root).unwrap_or(&path);
            if !includes
                .iter()
                .any(|pattern| pattern_matches(pattern, relative))
            {
                continue;
            }
            if exclude_patterns
                .iter()
                .any(|pattern| pattern_matches(pattern, relative))
            {
                continue;
            }
            add_file(&path, &mut seen, &mut selected)?;
        }
    }

    selected.sort();
    Ok(selected)
}

pub fn read_csv_trades(
    path: &Path,
    mapping: &CsvTradeMapping,
    delimiter: u8,
) -> Result<Vec<Trade>, Box<dyn Error>> {
    let file = File::open(path)?;
    let mut lines = BufReader::new(file).lines();
    let header = lines.next().ok_or("CSV file is empty")??;
    let headers = parse_csv_line(&header, delimiter);
    let header_index: HashMap<&str, usize> = headers
        .iter()
        .enumerate()
        .map(|(index, name)| (name.as_str(), index))
        .collect();
    let mut trades = Vec::new();
    for line in lines {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let fields = parse_csv_line(&line, delimiter);
        trades.push(trade_from_fields(&fields, &header_index, mapping)?);
    }
    Ok(trades)
}

pub fn aggregate_csv_files(
    paths: &[PathBuf],
    mapping: &CsvTradeMapping,
    interval_seconds: i64,
    fill_gaps: bool,
    delimiter: u8,
) -> Result<Vec<Candle>, Box<dyn Error>> {
    let mut trades = Vec::new();
    for path in paths {
        trades.extend(read_csv_trades(path, mapping, delimiter)?);
    }
    Ok(aggregate_trades(trades, interval_seconds, fill_gaps))
}

pub fn write_candles_csv(candles: &[Candle], output: Option<&Path>) -> Result<(), Box<dyn Error>> {
    let mut writer: Box<dyn Write> = match output {
        Some(path) => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            Box::new(File::create(path)?)
        }
        None => Box::new(std::io::stdout()),
    };
    writeln!(
        writer,
        "symbol,ts,datetime,open,high,low,close,volume,turnover,trade_count,buy_volume,sell_volume,first_trade_ts_ms,last_trade_ts_ms"
    )?;
    for candle in candles {
        writeln!(
            writer,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            candle.symbol,
            candle.ts,
            candle.datetime(),
            candle.open,
            candle.high,
            candle.low,
            candle.close,
            candle.volume,
            candle.turnover,
            candle.trade_count,
            candle.buy_volume,
            candle.sell_volume,
            optional_i64(candle.first_trade_ts_ms),
            optional_i64(candle.last_trade_ts_ms),
        )?;
    }
    Ok(())
}

pub fn write_candles_parquet(candles: &[Candle], output: &Path) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }

    let schema = Arc::new(Schema::new(vec![
        Field::new("symbol", DataType::Utf8, false),
        Field::new("ts", DataType::Int64, false),
        Field::new("datetime", DataType::Utf8, false),
        Field::new("open", DataType::Float64, false),
        Field::new("high", DataType::Float64, false),
        Field::new("low", DataType::Float64, false),
        Field::new("close", DataType::Float64, false),
        Field::new("volume", DataType::Float64, false),
        Field::new("turnover", DataType::Float64, false),
        Field::new("trade_count", DataType::UInt64, false),
        Field::new("buy_volume", DataType::Float64, false),
        Field::new("sell_volume", DataType::Float64, false),
        Field::new("first_trade_ts_ms", DataType::Int64, true),
        Field::new("last_trade_ts_ms", DataType::Int64, true),
    ]));

    let arrays: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from_iter_values(
            candles.iter().map(|candle| candle.symbol.as_str()),
        )),
        Arc::new(Int64Array::from_iter_values(candles.iter().map(|c| c.ts))),
        Arc::new(StringArray::from_iter_values(
            candles.iter().map(|candle| candle.datetime()),
        )),
        Arc::new(Float64Array::from_iter_values(
            candles.iter().map(|c| c.open),
        )),
        Arc::new(Float64Array::from_iter_values(
            candles.iter().map(|c| c.high),
        )),
        Arc::new(Float64Array::from_iter_values(
            candles.iter().map(|c| c.low),
        )),
        Arc::new(Float64Array::from_iter_values(
            candles.iter().map(|c| c.close),
        )),
        Arc::new(Float64Array::from_iter_values(
            candles.iter().map(|c| c.volume),
        )),
        Arc::new(Float64Array::from_iter_values(
            candles.iter().map(|c| c.turnover),
        )),
        Arc::new(UInt64Array::from_iter_values(
            candles.iter().map(|c| c.trade_count),
        )),
        Arc::new(Float64Array::from_iter_values(
            candles.iter().map(|c| c.buy_volume),
        )),
        Arc::new(Float64Array::from_iter_values(
            candles.iter().map(|c| c.sell_volume),
        )),
        Arc::new(Int64Array::from(
            candles
                .iter()
                .map(|c| c.first_trade_ts_ms)
                .collect::<Vec<_>>(),
        )),
        Arc::new(Int64Array::from(
            candles
                .iter()
                .map(|c| c.last_trade_ts_ms)
                .collect::<Vec<_>>(),
        )),
    ];
    let batch = RecordBatch::try_new(schema.clone(), arrays)?;
    let file = File::create(output)?;
    let props = WriterProperties::builder().build();
    let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;
    writer.write(&batch)?;
    writer.close()?;
    Ok(())
}

fn trade_from_fields(
    fields: &[String],
    header_index: &HashMap<&str, usize>,
    mapping: &CsvTradeMapping,
) -> Result<Trade, Box<dyn Error>> {
    let symbol = match (&mapping.symbol, &mapping.symbol_column) {
        (Some(symbol), _) => symbol.to_uppercase(),
        (None, Some(column)) => get_field(fields, header_index, column)?.to_uppercase(),
        (None, None) => return Err("CSV mapping requires symbol or symbol column".into()),
    };
    let ts_raw = get_field(fields, header_index, &mapping.timestamp_column)?;
    let mut ts_ms = ts_raw.parse::<f64>()?;
    if matches!(mapping.timestamp_unit, TimestampUnit::Seconds) {
        ts_ms *= 1000.0;
    }
    let price = get_field(fields, header_index, &mapping.price_column)?.parse::<f64>()?;
    let size = get_field(fields, header_index, &mapping.size_column)?.parse::<f64>()?;
    let turnover = optional_field(fields, header_index, mapping.turnover_column.as_deref())
        .map(str::parse::<f64>)
        .transpose()?;
    let side =
        optional_field(fields, header_index, mapping.side_column.as_deref()).map(str::to_string);
    let trade_id = optional_field(fields, header_index, mapping.trade_id_column.as_deref())
        .map(str::to_string);
    Ok(Trade {
        symbol,
        ts_ms: ts_ms as i64,
        price,
        size,
        turnover,
        side,
        trade_id,
    })
}

fn parse_csv_line(line: &str, delimiter: u8) -> Vec<String> {
    let delimiter = delimiter as char;
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;
    while let Some(ch) = chars.next() {
        if ch == '"' {
            if in_quotes && chars.peek() == Some(&'"') {
                current.push('"');
                chars.next();
            } else {
                in_quotes = !in_quotes;
            }
        } else if ch == delimiter && !in_quotes {
            fields.push(current);
            current = String::new();
        } else {
            current.push(ch);
        }
    }
    fields.push(current);
    fields
}

fn get_field<'a>(
    fields: &'a [String],
    header_index: &HashMap<&str, usize>,
    column: &str,
) -> Result<&'a str, Box<dyn Error>> {
    let index = header_index
        .get(column)
        .ok_or_else(|| format!("missing CSV column: {column}"))?;
    fields
        .get(*index)
        .map(|value| value.as_str())
        .ok_or_else(|| format!("missing CSV field for column: {column}").into())
}

fn optional_field<'a>(
    fields: &'a [String],
    header_index: &HashMap<&str, usize>,
    column: Option<&str>,
) -> Option<&'a str> {
    let column = column?;
    let index = header_index.get(column)?;
    let value = fields.get(*index)?;
    if value.is_empty() {
        None
    } else {
        Some(value.as_str())
    }
}

fn add_file(
    path: &Path,
    seen: &mut BTreeSet<PathBuf>,
    selected: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn Error>> {
    if !path.is_file() {
        return Ok(());
    }
    let canonical = path.canonicalize()?;
    if seen.insert(canonical) {
        selected.push(path.to_path_buf());
    }
    Ok(())
}

fn walk_files(root: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(path) = stack.pop() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn pattern_matches(pattern: &str, path: &Path) -> bool {
    let text = path.to_string_lossy().replace('\\', "/");
    if let Some(suffix) = pattern.strip_prefix("**/*") {
        return text.ends_with(suffix);
    }
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return text == prefix || text.starts_with(&format!("{prefix}/"));
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        return text.ends_with(suffix);
    }
    text == pattern
}

fn trade_id_sort_key(trade: &Trade) -> (u8, String) {
    match trade.trade_id.as_deref() {
        Some(id) if id.chars().all(|ch| ch.is_ascii_digit()) => {
            (0, format!("{:020}", id.parse::<u128>().unwrap_or(0)))
        }
        Some(id) => (1, id.to_string()),
        None => (1, String::new()),
    }
}

fn optional_i64(value: Option<i64>) -> String {
    value.map(|item| item.to_string()).unwrap_or_default()
}

fn format_unix_seconds(ts: i64) -> String {
    let days = ts.div_euclid(86_400);
    let seconds_of_day = ts.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 }.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096).div_euclid(365);
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2).div_euclid(153);
    let d = doy - (153 * mp + 2).div_euclid(5) + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    (year, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_gap_candles() {
        let candles = aggregate_trades(
            vec![
                Trade {
                    symbol: "abc".to_string(),
                    ts_ms: 0,
                    price: 10.0,
                    size: 1.0,
                    turnover: None,
                    side: Some("buy".to_string()),
                    trade_id: Some("1".to_string()),
                },
                Trade {
                    symbol: "abc".to_string(),
                    ts_ms: 120_000,
                    price: 12.0,
                    size: 2.0,
                    turnover: None,
                    side: Some("sell".to_string()),
                    trade_id: Some("2".to_string()),
                },
            ],
            60,
            true,
        );
        assert_eq!(candles.len(), 3);
        assert_eq!(candles[1].ts, 60);
        assert_eq!(candles[1].open, 10.0);
        assert_eq!(candles[1].volume, 0.0);
        assert_eq!(candles[1].trade_count, 0);
    }

    #[test]
    fn parses_interval_strings() {
        assert_eq!(parse_interval_seconds("60").unwrap(), 60);
        assert_eq!(parse_interval_seconds("5m").unwrap(), 300);
        assert_eq!(parse_interval_seconds("2h").unwrap(), 7_200);
        assert!(parse_interval_seconds("0m").is_err());
    }

    #[test]
    fn formats_utc_datetime() {
        assert_eq!(format_unix_seconds(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_unix_seconds(60), "1970-01-01T00:01:00Z");
    }
}
