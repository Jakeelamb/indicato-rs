use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use ta_indicators::tradingview::{
    Candle,
    validated::{
        Side, WyckoffPhase, adaptive_baseline, anchored_vwap, darvas_turtle_breakout,
        heikin_ashi_transform, ichimoku_cloud_state, mariashi_renko_system, momentum_vol_composite,
        self_strength_oscillator, volumetric_trend_ribbon, wyckoff_phase,
        xwisetrade_volume_liquidity_sweep,
    },
};

const DEFAULT_GOLDEN_FIXTURE_ROWS: usize = 180;
const EXPECTED_DIR: &str = "tests/fixtures/tradingview_expected";
const OHLCV_FILE: &str = "ohlcv.csv";
const TOLERANCE: f64 = 1e-6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TradingViewGoldenCase {
    family: &'static str,
    file_name: &'static str,
    columns: &'static [&'static str],
}

const SOURCE_ALIGNED_GOLDEN_CASES: &[TradingViewGoldenCase] = &[
    TradingViewGoldenCase {
        family: "volume_liquidity_sweep",
        file_name: "volume_liquidity_sweep.csv",
        columns: &[
            "bar_index",
            "bull_signal",
            "bear_signal",
            "bull_level",
            "bear_level",
        ],
    },
    TradingViewGoldenCase {
        family: "volumetric_trend_ribbon",
        file_name: "volumetric_trend_ribbon.csv",
        columns: &[
            "bar_index",
            "base",
            "vwsd",
            "upper",
            "lower",
            "side",
            "bull_cross",
            "bear_cross",
        ],
    },
    TradingViewGoldenCase {
        family: "momentum_vol_composite",
        file_name: "momentum_vol_composite.csv",
        columns: &[
            "bar_index",
            "roc_z",
            "atr_z",
            "volume_flow_z",
            "ema_diff_z",
            "composite",
            "composite_ma",
            "slope",
            "signal",
        ],
    },
    TradingViewGoldenCase {
        family: "adaptive_baseline",
        file_name: "adaptive_baseline.csv",
        columns: &[
            "bar_index",
            "tpi_score",
            "score_1",
            "score_2",
            "score_3",
            "score_4",
            "score_5",
        ],
    },
    TradingViewGoldenCase {
        family: "self_strength_oscillator",
        file_name: "self_strength_oscillator.csv",
        columns: &[
            "bar_index",
            "baseline",
            "strength",
            "signal",
            "histogram",
            "new_strength_high",
            "cross_up",
            "cross_down",
        ],
    },
    TradingViewGoldenCase {
        family: "mariashi_renko_system",
        file_name: "mariashi_renko_system.csv",
        columns: &[
            "bar_index",
            "virtual_open",
            "virtual_close",
            "brick_size",
            "signal",
            "trailing_stop",
            "consecutive_bricks",
        ],
    },
    TradingViewGoldenCase {
        family: "anchored_vwap",
        file_name: "anchored_vwap.csv",
        columns: &[
            "bar_index",
            "anchor_index",
            "vwap",
            "sigma",
            "upper_band",
            "lower_band",
            "distance_pct",
            "signal",
        ],
    },
    TradingViewGoldenCase {
        family: "wyckoff_phase",
        file_name: "wyckoff_phase.csv",
        columns: &[
            "bar_index",
            "range_high",
            "range_low",
            "range_position",
            "volume_ratio",
            "trend_slope_pct",
            "phase",
        ],
    },
    TradingViewGoldenCase {
        family: "darvas_turtle_breakout",
        file_name: "darvas_turtle_breakout.csv",
        columns: &[
            "bar_index",
            "box_high",
            "box_low",
            "breakout",
            "stop",
            "risk_pct",
            "bars_since_breakout",
        ],
    },
    TradingViewGoldenCase {
        family: "ichimoku_cloud_state",
        file_name: "ichimoku_cloud_state.csv",
        columns: &[
            "bar_index",
            "tenkan",
            "kijun",
            "senkou_a",
            "senkou_b",
            "cloud_top",
            "cloud_bottom",
            "cloud_thickness_pct",
            "price_state",
            "tk_cross",
            "kumo_twist",
        ],
    },
    TradingViewGoldenCase {
        family: "heikin_ashi_transform",
        file_name: "heikin_ashi_transform.csv",
        columns: &[
            "bar_index",
            "ha_open",
            "ha_high",
            "ha_low",
            "ha_close",
            "color",
            "body_pct",
            "upper_wick_pct",
            "lower_wick_pct",
        ],
    },
];

#[derive(Debug, Clone, Copy)]
struct GoldenCase {
    family: &'static str,
    compute: fn(&[Candle]) -> Vec<BTreeMap<String, String>>,
}

const CASES: &[GoldenCase] = &[
    GoldenCase {
        family: "volume_liquidity_sweep",
        compute: volume_liquidity_sweep_rows,
    },
    GoldenCase {
        family: "volumetric_trend_ribbon",
        compute: volumetric_trend_ribbon_rows,
    },
    GoldenCase {
        family: "momentum_vol_composite",
        compute: momentum_vol_composite_rows,
    },
    GoldenCase {
        family: "adaptive_baseline",
        compute: adaptive_baseline_rows,
    },
    GoldenCase {
        family: "self_strength_oscillator",
        compute: self_strength_oscillator_rows,
    },
    GoldenCase {
        family: "mariashi_renko_system",
        compute: mariashi_renko_system_rows,
    },
    GoldenCase {
        family: "anchored_vwap",
        compute: anchored_vwap_rows,
    },
    GoldenCase {
        family: "wyckoff_phase",
        compute: wyckoff_phase_rows,
    },
    GoldenCase {
        family: "darvas_turtle_breakout",
        compute: darvas_turtle_breakout_rows,
    },
    GoldenCase {
        family: "ichimoku_cloud_state",
        compute: ichimoku_cloud_state_rows,
    },
    GoldenCase {
        family: "heikin_ashi_transform",
        compute: heikin_ashi_transform_rows,
    },
];

#[test]
fn golden_families_have_expected_or_pending_status() {
    let pending = pending_families();
    let golden_cases = CASES
        .iter()
        .map(|case| case.family)
        .collect::<BTreeSet<_>>();
    let manifest_cases = SOURCE_ALIGNED_GOLDEN_CASES
        .iter()
        .map(|case| case.family)
        .collect::<BTreeSet<_>>();

    assert_eq!(
        manifest_cases, golden_cases,
        "every golden manifest family needs a golden harness case"
    );

    for case in CASES {
        let schema = schema(case.family);
        let expected_path = Path::new(EXPECTED_DIR).join(schema.file_name);
        assert!(
            expected_path.exists() || pending.contains(case.family),
            "{} needs either {} or a pending.tsv blocker",
            case.family,
            expected_path.display()
        );
    }
}

#[test]
fn golden_row_generators_emit_the_declared_schema_columns() {
    let candles = generated_fixture_candles(DEFAULT_GOLDEN_FIXTURE_ROWS);

    for case in CASES {
        let schema = schema(case.family);
        let declared = schema.columns.iter().copied().collect::<BTreeSet<_>>();
        let actual_rows = (case.compute)(&candles);

        for (row_idx, row) in actual_rows.iter().enumerate() {
            let actual = row.keys().map(String::as_str).collect::<BTreeSet<_>>();
            assert_eq!(
                actual, declared,
                "{} row {row_idx} does not match the declared golden schema",
                case.family
            );
        }
    }
}

#[test]
fn tradingview_golden_exports_match_when_present() {
    let candles = fixture_candles();
    let pending = pending_families();

    for case in CASES {
        let schema = schema(case.family);
        let expected_path = Path::new(EXPECTED_DIR).join(schema.file_name);
        if !expected_path.exists() {
            assert!(
                pending.contains(case.family),
                "{} has no expected CSV and no pending blocker",
                case.family
            );
            continue;
        }

        let expected = read_expected_csv(&expected_path);
        let comparable_columns = comparable_indicator_columns(schema, &expected)
            .unwrap_or_else(|message| panic!("{message}"));
        let actual = (case.compute)(&candles);
        assert_eq!(
            expected.len(),
            actual.len(),
            "{} row count mismatch",
            case.family
        );

        for (row_idx, (expected_row, actual_row)) in expected.iter().zip(&actual).enumerate() {
            for column in &comparable_columns {
                let expected_value = expected_row
                    .get(*column)
                    .unwrap_or_else(|| panic!("{} missing expected column {column}", case.family));
                let actual_value = actual_row
                    .get(*column)
                    .unwrap_or_else(|| panic!("{} missing actual column {column}", case.family));
                assert_cell_close(case.family, row_idx, column, expected_value, actual_value);
            }
        }
    }
}

#[test]
fn expected_indicator_csvs_may_include_extra_tradingview_export_columns() {
    let schema = schema("volume_liquidity_sweep");
    let expected = vec![BTreeMap::from([
        ("time".to_string(), "2026-06-18".to_string()),
        ("open".to_string(), "100".to_string()),
        ("bar_index".to_string(), "0".to_string()),
        ("bull_signal".to_string(), "1".to_string()),
    ])];

    assert_eq!(
        comparable_indicator_columns(schema, &expected).expect("bull_signal should be comparable"),
        vec!["bull_signal"]
    );
}

#[test]
fn expected_indicator_csvs_need_at_least_one_declared_indicator_column() {
    let schema = schema("volume_liquidity_sweep");
    let expected = vec![BTreeMap::from([
        ("time".to_string(), "2026-06-18".to_string()),
        ("open".to_string(), "100".to_string()),
        ("bar_index".to_string(), "0".to_string()),
    ])];

    let err = comparable_indicator_columns(schema, &expected)
        .expect_err("unrelated export columns should not be comparable");
    assert!(err.contains(
        "volume_liquidity_sweep expected CSV must include at least one declared indicator column"
    ));
}

#[test]
fn csv_parser_allows_quoted_commas_and_escaped_quotes() {
    let row = "time,\"1,234.50\",\"say \"\"hi\"\"\",plain,\"\"\"quoted\"\"\"";

    assert_eq!(
        parse_csv_row(row).expect("quoted CSV row should parse"),
        vec!["time", "1,234.50", "say \"hi\"", "plain", "\"quoted\""]
    );
}

#[test]
fn csv_parser_rejects_unterminated_quotes() {
    let err = parse_csv_row("time,\"1,234.50").expect_err("unterminated quote should fail");

    assert!(err.contains("unterminated quoted field"));
}

#[test]
fn csv_parser_rejects_quotes_inside_unquoted_fields() {
    let err = parse_csv_row("time,12\"34").expect_err("unquoted quote should fail");

    assert!(err.contains("quote inside unquoted field"));
}

#[test]
fn cell_comparator_allows_comma_formatted_numbers() {
    assert_cell_close("family", 0, "level", "1,234.5000001", "1234.5");
}

#[test]
fn cell_comparator_still_rejects_nonnumeric_mismatches() {
    let err = std::panic::catch_unwind(|| {
        assert_cell_close("family", 0, "state", "pending", "ready");
    });

    assert!(err.is_err());
}

fn schema(family: &str) -> &'static TradingViewGoldenCase {
    SOURCE_ALIGNED_GOLDEN_CASES
        .iter()
        .find(|case| case.family == family)
        .unwrap_or_else(|| panic!("{family} is missing from SOURCE_ALIGNED_GOLDEN_CASES"))
}

fn pending_families() -> BTreeSet<String> {
    let path = Path::new(EXPECTED_DIR).join("pending.tsv");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    text.lines()
        .skip(1)
        .filter_map(|line| line.split_once('\t').map(|(family, _)| family.to_string()))
        .collect()
}

fn read_expected_csv(path: &Path) -> Vec<BTreeMap<String, String>> {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    let mut lines = text.lines();
    let header_line = lines
        .next()
        .unwrap_or_else(|| panic!("{} is empty", path.display()));
    let header = parse_csv_row(header_line)
        .unwrap_or_else(|err| panic!("failed to parse {} header: {err}", path.display()));

    lines
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(line_idx, line)| {
            let values = parse_csv_row(line).unwrap_or_else(|err| {
                panic!(
                    "failed to parse {} line {}: {err}",
                    path.display(),
                    line_idx + 2
                )
            });
            assert_eq!(
                header.len(),
                values.len(),
                "{} column count mismatch on line {line}",
                path.display()
            );
            header
                .iter()
                .cloned()
                .zip(values)
                .collect::<BTreeMap<String, String>>()
        })
        .collect()
}

fn parse_csv_row(line: &str) -> Result<Vec<String>, String> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;
    let mut after_quote = false;

    while let Some(ch) = chars.next() {
        if in_quotes {
            match ch {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    field.push('"');
                }
                '"' => {
                    in_quotes = false;
                    after_quote = true;
                }
                _ => field.push(ch),
            }
            continue;
        }

        if after_quote {
            match ch {
                ',' => {
                    fields.push(std::mem::take(&mut field));
                    after_quote = false;
                }
                ' ' | '\t' => {}
                _ => return Err(format!("unexpected character {ch:?} after closing quote")),
            }
            continue;
        }

        match ch {
            ',' => fields.push(std::mem::take(&mut field)),
            '"' if field.is_empty() => in_quotes = true,
            '"' => return Err("quote inside unquoted field".to_string()),
            _ => field.push(ch),
        }
    }

    if in_quotes {
        return Err("unterminated quoted field".to_string());
    }

    fields.push(field);
    Ok(fields)
}

fn comparable_indicator_columns<'a>(
    schema: &'a TradingViewGoldenCase,
    expected: &[BTreeMap<String, String>],
) -> Result<Vec<&'a str>, String> {
    let Some(first_row) = expected.first() else {
        return Ok(schema
            .columns
            .iter()
            .copied()
            .filter(|column| *column != "bar_index")
            .collect());
    };
    let comparable = schema
        .columns
        .iter()
        .copied()
        .filter(|column| *column != "bar_index" && first_row.contains_key(*column))
        .collect::<Vec<_>>();
    if comparable.is_empty() {
        return Err(format!(
            "{} expected CSV must include at least one declared indicator column; declared columns: {}",
            schema.family,
            schema.columns.join(",")
        ));
    }
    Ok(comparable)
}

fn required_number(row: &BTreeMap<String, String>, aliases: &[&str]) -> f64 {
    for alias in aliases {
        if let Some(value) = row
            .iter()
            .find(|(column, _)| column.eq_ignore_ascii_case(alias))
            .map(|(_, value)| value)
        {
            return value
                .replace(',', "")
                .parse::<f64>()
                .unwrap_or_else(|err| panic!("failed to parse {alias} value {value}: {err}"));
        }
    }

    panic!("missing required OHLCV column; tried {aliases:?}");
}

fn assert_cell_close(family: &str, row_idx: usize, column: &str, expected: &str, actual: &str) {
    if expected == actual {
        return;
    }
    let expected_number = parse_number_cell(expected);
    let actual_number = parse_number_cell(actual);
    if let (Ok(expected_number), Ok(actual_number)) = (expected_number, actual_number) {
        if expected_number.is_nan() && actual_number.is_nan() {
            return;
        }
        assert!(
            (expected_number - actual_number).abs() <= TOLERANCE,
            "{family} row {row_idx} column {column}: expected {expected}, got {actual}"
        );
    } else {
        panic!("{family} row {row_idx} column {column}: expected {expected}, got {actual}");
    }
}

fn parse_number_cell(value: &str) -> Result<f64, std::num::ParseFloatError> {
    value.replace(',', "").parse::<f64>()
}

fn fixture_candles() -> Vec<Candle> {
    let has_expected_export = CASES.iter().any(|case| {
        Path::new(EXPECTED_DIR)
            .join(schema(case.family).file_name)
            .exists()
    });
    if has_expected_export {
        return tradingview_ohlcv_candles();
    }

    generated_fixture_candles(DEFAULT_GOLDEN_FIXTURE_ROWS)
}

fn tradingview_ohlcv_candles() -> Vec<Candle> {
    let path = Path::new(EXPECTED_DIR).join(OHLCV_FILE);
    assert!(
        path.exists(),
        "TradingView golden CSVs require matching {}",
        path.display()
    );

    read_expected_csv(&path)
        .into_iter()
        .map(|row| {
            Candle::new(
                required_number(&row, &["open"]),
                required_number(&row, &["high"]),
                required_number(&row, &["low"]),
                required_number(&row, &["close"]),
                required_number(&row, &["volume", "vol"]),
            )
        })
        .collect()
}

fn generated_fixture_candles(rows: usize) -> Vec<Candle> {
    (0..rows)
        .map(|idx| {
            let idxf = idx as f64;
            let wave = (idxf * 0.17).sin() * 2.6 + (idxf * 0.07).cos() * 1.4;
            let close = 100.0 + idxf * 0.08 + wave;
            let open = close - (idxf * 0.11).sin() * 0.8;
            let high = open.max(close) + 0.9 + (idx % 5) as f64 * 0.08;
            let low = open.min(close) - 0.85 - (idx % 7) as f64 * 0.06;
            let volume = 2_000.0 + (idx % 17) as f64 * 115.0 + (idxf * 0.13).sin().abs() * 400.0;
            Candle::new(open, high, low, close, volume)
        })
        .collect()
}

fn row(index: usize) -> BTreeMap<String, String> {
    BTreeMap::from([("bar_index".to_string(), index.to_string())])
}

fn put_number(row: &mut BTreeMap<String, String>, column: &str, value: f64) {
    let cell = if value.is_finite() {
        format!("{value:.10}")
    } else {
        String::new()
    };
    row.insert(column.to_string(), cell);
}

fn put_bool(row: &mut BTreeMap<String, String>, column: &str, value: bool) {
    row.insert(
        column.to_string(),
        if value { "1" } else { "0" }.to_string(),
    );
}

fn put_side(row: &mut BTreeMap<String, String>, column: &str, side: Option<Side>) {
    let value = match side {
        Some(Side::Bullish) => "1",
        Some(Side::Bearish) => "-1",
        None => "0",
    };
    row.insert(column.to_string(), value.to_string());
}

fn volume_liquidity_sweep_rows(candles: &[Candle]) -> Vec<BTreeMap<String, String>> {
    let signals = xwisetrade_volume_liquidity_sweep(candles, 5, 20, 1.5, 3);
    signals
        .iter()
        .enumerate()
        .map(|(idx, signals)| {
            let mut row = row(idx);
            let bull = signals.iter().find(|signal| signal.side == Side::Bullish);
            let bear = signals.iter().find(|signal| signal.side == Side::Bearish);
            put_bool(&mut row, "bull_signal", bull.is_some());
            put_bool(&mut row, "bear_signal", bear.is_some());
            put_number(
                &mut row,
                "bull_level",
                bull.map(|s| s.level).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "bear_level",
                bear.map(|s| s.level).unwrap_or(f64::NAN),
            );
            row
        })
        .collect()
}

fn volumetric_trend_ribbon_rows(candles: &[Candle]) -> Vec<BTreeMap<String, String>> {
    let points = volumetric_trend_ribbon(candles);
    points
        .iter()
        .enumerate()
        .map(|(idx, point)| {
            let mut row = row(idx);
            put_number(&mut row, "base", point.map(|p| p.fast).unwrap_or(f64::NAN));
            put_number(&mut row, "vwsd", point.map(|p| p.slow).unwrap_or(f64::NAN));
            put_number(
                &mut row,
                "upper",
                point.map(|p| p.upper).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "lower",
                point.map(|p| p.lower).unwrap_or(f64::NAN),
            );
            put_side(&mut row, "side", point.and_then(|p| p.side));
            put_bool(&mut row, "bull_cross", point.is_some_and(|p| p.bull_cross));
            put_bool(&mut row, "bear_cross", point.is_some_and(|p| p.bear_cross));
            row
        })
        .collect()
}

fn momentum_vol_composite_rows(candles: &[Candle]) -> Vec<BTreeMap<String, String>> {
    let points = momentum_vol_composite(candles);
    points
        .iter()
        .enumerate()
        .map(|(idx, point)| {
            let mut row = row(idx);
            put_number(
                &mut row,
                "roc_z",
                point.map(|p| p.momentum_z).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "atr_z",
                point.map(|p| p.volatility_z).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "volume_flow_z",
                point.map(|p| p.volume_z).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "ema_diff_z",
                point.map(|p| p.ema_diff_z).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "composite",
                point.map(|p| p.score).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "composite_ma",
                point.map(|p| p.composite_ma).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "slope",
                point.map(|p| p.slope).unwrap_or(f64::NAN),
            );
            put_side(&mut row, "signal", point.and_then(|p| p.signal));
            row
        })
        .collect()
}

fn adaptive_baseline_rows(candles: &[Candle]) -> Vec<BTreeMap<String, String>> {
    let points = adaptive_baseline(candles);
    points
        .iter()
        .enumerate()
        .map(|(idx, point)| {
            let mut row = row(idx);
            put_number(
                &mut row,
                "tpi_score",
                point.map(|p| p.value).unwrap_or(f64::NAN),
            );
            if let Some(point) = point {
                for score_idx in 0..5 {
                    row.insert(
                        format!("score_{}", score_idx + 1),
                        point.scores[score_idx].to_string(),
                    );
                }
            } else {
                for score_idx in 0..5 {
                    row.insert(format!("score_{}", score_idx + 1), String::new());
                }
            }
            row
        })
        .collect()
}

fn self_strength_oscillator_rows(candles: &[Candle]) -> Vec<BTreeMap<String, String>> {
    let points = self_strength_oscillator(candles);
    points
        .iter()
        .enumerate()
        .map(|(idx, point)| {
            let mut row = row(idx);
            put_number(
                &mut row,
                "baseline",
                point.map(|p| p.baseline).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "strength",
                point.map(|p| p.strength).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "signal",
                point.map(|p| p.signal).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "histogram",
                point.map(|p| p.histogram).unwrap_or(f64::NAN),
            );
            put_bool(
                &mut row,
                "new_strength_high",
                point.is_some_and(|p| p.new_strength_high),
            );
            put_bool(&mut row, "cross_up", point.is_some_and(|p| p.cross_up));
            put_bool(&mut row, "cross_down", point.is_some_and(|p| p.cross_down));
            row
        })
        .collect()
}

fn mariashi_renko_system_rows(candles: &[Candle]) -> Vec<BTreeMap<String, String>> {
    let points = mariashi_renko_system(candles);
    points
        .iter()
        .enumerate()
        .map(|(idx, point)| {
            let mut row = row(idx);
            put_number(
                &mut row,
                "virtual_open",
                point.map(|p| p.virtual_open).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "virtual_close",
                point.map(|p| p.virtual_close).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "brick_size",
                point.map(|p| p.brick_size).unwrap_or(f64::NAN),
            );
            put_side(&mut row, "signal", point.and_then(|p| p.signal));
            put_number(
                &mut row,
                "trailing_stop",
                point.and_then(|p| p.trailing_stop).unwrap_or(f64::NAN),
            );
            row.insert(
                "consecutive_bricks".to_string(),
                point.map_or_else(String::new, |p| p.consecutive_bricks.to_string()),
            );
            row
        })
        .collect()
}

fn anchored_vwap_rows(candles: &[Candle]) -> Vec<BTreeMap<String, String>> {
    let anchors = (0..candles.len())
        .map(|idx| idx % 40 == 0)
        .collect::<Vec<_>>();
    let points = anchored_vwap(candles, &anchors, 2.0);
    points
        .iter()
        .enumerate()
        .map(|(idx, point)| {
            let mut row = row(idx);
            row.insert(
                "anchor_index".to_string(),
                point.map_or_else(String::new, |p| p.anchor_index.to_string()),
            );
            put_number(&mut row, "vwap", point.map(|p| p.vwap).unwrap_or(f64::NAN));
            put_number(
                &mut row,
                "sigma",
                point.map(|p| p.sigma).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "upper_band",
                point.map(|p| p.upper_band).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "lower_band",
                point.map(|p| p.lower_band).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "distance_pct",
                point.map(|p| p.distance_pct).unwrap_or(f64::NAN),
            );
            put_side(&mut row, "signal", point.and_then(|p| p.signal));
            row
        })
        .collect()
}

fn wyckoff_phase_rows(candles: &[Candle]) -> Vec<BTreeMap<String, String>> {
    let points = wyckoff_phase(candles, 20, 14);
    points
        .iter()
        .enumerate()
        .map(|(idx, point)| {
            let mut row = row(idx);
            put_number(
                &mut row,
                "range_high",
                point.map(|p| p.range_high).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "range_low",
                point.map(|p| p.range_low).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "range_position",
                point.map(|p| p.range_position).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "volume_ratio",
                point.map(|p| p.volume_ratio).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "trend_slope_pct",
                point.map(|p| p.trend_slope_pct).unwrap_or(f64::NAN),
            );
            row.insert(
                "phase".to_string(),
                point.map_or_else(
                    || "0".to_string(),
                    |p| wyckoff_phase_code(p.phase).to_string(),
                ),
            );
            row
        })
        .collect()
}

fn darvas_turtle_breakout_rows(candles: &[Candle]) -> Vec<BTreeMap<String, String>> {
    let points = darvas_turtle_breakout(candles, 20);
    points
        .iter()
        .enumerate()
        .map(|(idx, point)| {
            let mut row = row(idx);
            put_number(
                &mut row,
                "box_high",
                point.map(|p| p.box_high).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "box_low",
                point.map(|p| p.box_low).unwrap_or(f64::NAN),
            );
            put_side(&mut row, "breakout", point.and_then(|p| p.breakout));
            put_number(&mut row, "stop", point.map(|p| p.stop).unwrap_or(f64::NAN));
            put_number(
                &mut row,
                "risk_pct",
                point.map(|p| p.risk_pct).unwrap_or(f64::NAN),
            );
            row.insert(
                "bars_since_breakout".to_string(),
                point
                    .and_then(|p| p.bars_since_breakout)
                    .map_or_else(String::new, |age| age.to_string()),
            );
            row
        })
        .collect()
}

fn ichimoku_cloud_state_rows(candles: &[Candle]) -> Vec<BTreeMap<String, String>> {
    let points = ichimoku_cloud_state(candles, 9, 26, 52);
    points
        .iter()
        .enumerate()
        .map(|(idx, point)| {
            let mut row = row(idx);
            put_number(
                &mut row,
                "tenkan",
                point.map(|p| p.tenkan).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "kijun",
                point.map(|p| p.kijun).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "senkou_a",
                point.map(|p| p.senkou_a).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "senkou_b",
                point.map(|p| p.senkou_b).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "cloud_top",
                point.map(|p| p.cloud_top).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "cloud_bottom",
                point.map(|p| p.cloud_bottom).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "cloud_thickness_pct",
                point.map(|p| p.cloud_thickness_pct).unwrap_or(f64::NAN),
            );
            put_side(&mut row, "price_state", point.and_then(|p| p.price_state));
            put_side(&mut row, "tk_cross", point.and_then(|p| p.tk_cross));
            put_side(&mut row, "kumo_twist", point.and_then(|p| p.kumo_twist));
            row
        })
        .collect()
}

fn heikin_ashi_transform_rows(candles: &[Candle]) -> Vec<BTreeMap<String, String>> {
    let points = heikin_ashi_transform(candles);
    points
        .iter()
        .enumerate()
        .map(|(idx, point)| {
            let mut row = row(idx);
            put_number(
                &mut row,
                "ha_open",
                point.map(|p| p.open).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "ha_high",
                point.map(|p| p.high).unwrap_or(f64::NAN),
            );
            put_number(&mut row, "ha_low", point.map(|p| p.low).unwrap_or(f64::NAN));
            put_number(
                &mut row,
                "ha_close",
                point.map(|p| p.close).unwrap_or(f64::NAN),
            );
            put_side(&mut row, "color", point.and_then(|p| p.color));
            put_number(
                &mut row,
                "body_pct",
                point.map(|p| p.body_pct).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "upper_wick_pct",
                point.map(|p| p.upper_wick_pct).unwrap_or(f64::NAN),
            );
            put_number(
                &mut row,
                "lower_wick_pct",
                point.map(|p| p.lower_wick_pct).unwrap_or(f64::NAN),
            );
            row
        })
        .collect()
}

fn wyckoff_phase_code(phase: WyckoffPhase) -> i32 {
    match phase {
        WyckoffPhase::Accumulation => 1,
        WyckoffPhase::Markup => 2,
        WyckoffPhase::Distribution => -1,
        WyckoffPhase::Markdown => -2,
        WyckoffPhase::Neutral => 0,
    }
}
