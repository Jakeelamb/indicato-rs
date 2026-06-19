use indicato_rs as ours;
use serde_json::Value;
use std::hint::black_box;
use std::time::{Duration, Instant};
use talib_rs as upstream;
use upstream::MaType;

const REL_TOL: f64 = 1e-6;
const ABS_TOL: f64 = 1e-6;

struct Inputs {
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
    close2: Vec<f64>,
}

fn load_fixture() -> (Inputs, Value) {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/talib_parity.json"
    );
    let raw = std::fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("read parity fixture {path}: {err}"));
    let root: Value = serde_json::from_str(&raw).expect("parse parity fixture json");
    let input = &root["input"];
    let series = |key: &str| -> Vec<f64> {
        input[key]
            .as_array()
            .unwrap_or_else(|| panic!("missing input series {key}"))
            .iter()
            .map(|value| value.as_f64().expect("finite input value"))
            .collect()
    };
    (
        Inputs {
            open: series("open"),
            high: series("high"),
            low: series("low"),
            close: series("close"),
            volume: series("volume"),
            close2: series("close2"),
        },
        root,
    )
}

fn expected(root: &Value, key: &str) -> Vec<Option<f64>> {
    root["expected"][key]
        .as_array()
        .unwrap_or_else(|| panic!("missing expected series {key}"))
        .iter()
        .map(|value| {
            if value.is_null() {
                None
            } else {
                value.as_f64()
            }
        })
        .collect()
}

fn compare_f64_series(
    key: &str,
    actual: &[f64],
    expected: &[Option<f64>],
) -> Result<usize, String> {
    if actual.len() != expected.len() {
        return Err(format!(
            "{key}: length {} != expected {}",
            actual.len(),
            expected.len()
        ));
    }
    let mut compared = 0usize;
    for (idx, exp) in expected.iter().enumerate() {
        let Some(exp) = exp else { continue };
        compared += 1;
        let got = actual[idx];
        if got.is_nan() {
            return Err(format!("{key}: idx {idx}: got NaN, expected {exp:.10}"));
        }
        let diff = (got - exp).abs();
        let tol = ABS_TOL + REL_TOL * exp.abs();
        if diff > tol {
            return Err(format!(
                "{key}: idx {idx}: got {got:.10}, expected {exp:.10} (diff {diff:.3e} > tol {tol:.3e})"
            ));
        }
    }
    if compared == 0 {
        return Err(format!("{key}: no overlapping values"));
    }
    Ok(compared)
}

fn compare_i32_series(
    key: &str,
    actual: &[i32],
    expected: &[Option<f64>],
) -> Result<usize, String> {
    let actual = actual.iter().map(|value| *value as f64).collect::<Vec<_>>();
    compare_f64_series(key, &actual, expected)
}

#[test]
fn upstream_matches_our_talib_fixture_except_known_apo_ppo_divergence() {
    let (i, root) = load_fixture();
    let c = &i.close;
    let (macd, signal, hist) = upstream::momentum::macd(c, 12, 26, 9).expect("upstream macd");
    let (bb_upper, bb_middle, bb_lower) =
        upstream::overlap::bbands(c, 20, 2.0, 2.0, MaType::Sma).expect("upstream bbands");
    let (aroon_down, aroon_up) =
        upstream::momentum::aroon(&i.high, &i.low, 14).expect("upstream aroon");
    let (mama, fama) = upstream::overlap::mama(c, 0.5, 0.05).expect("upstream mama");
    let (ht_inphase, ht_quadrature) = upstream::cycle::ht_phasor(c).expect("upstream ht_phasor");
    let (ht_sine, ht_leadsine) = upstream::cycle::ht_sine(c).expect("upstream ht_sine");

    let cases: Vec<(&str, Vec<f64>)> = vec![
        (
            "ema_14",
            upstream::overlap::ema(c, 14).expect("upstream ema"),
        ),
        (
            "dema_14",
            upstream::overlap::dema(c, 14).expect("upstream dema"),
        ),
        (
            "tema_14",
            upstream::overlap::tema(c, 14).expect("upstream tema"),
        ),
        (
            "t3_5_0p7",
            upstream::overlap::t3(c, 5, 0.7).expect("upstream t3"),
        ),
        (
            "kama_30",
            upstream::overlap::kama(c, 30).expect("upstream kama"),
        ),
        (
            "trix_15",
            upstream::momentum::trix(c, 15).expect("upstream trix"),
        ),
        (
            "rsi_14",
            upstream::momentum::rsi(c, 14).expect("upstream rsi"),
        ),
        (
            "cmo_14",
            upstream::momentum::cmo(c, 14).expect("upstream cmo"),
        ),
        (
            "apo_12_26",
            upstream::momentum::apo(c, 12, 26, MaType::Sma).expect("upstream apo"),
        ),
        (
            "ppo_12_26",
            upstream::momentum::ppo(c, 12, 26, MaType::Sma).expect("upstream ppo"),
        ),
        (
            "ultosc_7_14_28",
            upstream::momentum::ultosc(&i.high, &i.low, c, 7, 14, 28).expect("upstream ultosc"),
        ),
        (
            "bop",
            upstream::momentum::bop(&i.open, &i.high, &i.low, c).expect("upstream bop"),
        ),
        ("macd_12_26_9", macd),
        ("macd_signal_12_26_9", signal),
        ("macd_hist_12_26_9", hist),
        ("bb_upper_20_2", bb_upper),
        ("bb_middle_20_2", bb_middle),
        ("bb_lower_20_2", bb_lower),
        (
            "adx_14",
            upstream::momentum::adx(&i.high, &i.low, c, 14).expect("upstream adx"),
        ),
        (
            "adxr_14",
            upstream::momentum::adxr(&i.high, &i.low, c, 14).expect("upstream adxr"),
        ),
        (
            "plus_di_14",
            upstream::momentum::plus_di(&i.high, &i.low, c, 14).expect("upstream plus_di"),
        ),
        (
            "minus_di_14",
            upstream::momentum::minus_di(&i.high, &i.low, c, 14).expect("upstream minus_di"),
        ),
        (
            "dx_14",
            upstream::momentum::dx(&i.high, &i.low, c, 14).expect("upstream dx"),
        ),
        ("aroon_up_14", aroon_up),
        ("aroon_down_14", aroon_down),
        (
            "aroon_osc_14",
            upstream::momentum::aroon_osc(&i.high, &i.low, 14).expect("upstream aroon_osc"),
        ),
        (
            "ad",
            upstream::volume::ad(&i.high, &i.low, c, &i.volume).expect("upstream ad"),
        ),
        (
            "adosc_3_10",
            upstream::volume::adosc(&i.high, &i.low, c, &i.volume, 3, 10).expect("upstream adosc"),
        ),
        (
            "sar_0p02_0p2",
            upstream::overlap::sar(&i.high, &i.low, 0.02, 0.2).expect("upstream sar"),
        ),
        (
            "sarext_default",
            upstream::overlap::sar_ext(&i.high, &i.low, 0.0, 0.0, 0.02, 0.02, 0.2, 0.02, 0.02, 0.2)
                .expect("upstream sarext"),
        ),
        (
            "sum_10",
            upstream::math_operator::sum(c, 10).expect("upstream sum"),
        ),
        (
            "beta_5",
            upstream::statistic::beta(c, &i.close2, 5).expect("upstream beta"),
        ),
        (
            "correl_30",
            upstream::statistic::correl(c, &i.close2, 30).expect("upstream correl"),
        ),
        ("mama_0p5_0p05", mama),
        ("fama_0p5_0p05", fama),
        (
            "ht_dcperiod",
            upstream::cycle::ht_dcperiod(c).expect("upstream ht_dcperiod"),
        ),
        ("ht_phasor_inphase", ht_inphase),
        ("ht_phasor_quadrature", ht_quadrature),
        (
            "ht_dcphase",
            upstream::cycle::ht_dcphase(c).expect("upstream ht_dcphase"),
        ),
        ("ht_sine", ht_sine),
        ("ht_leadsine", ht_leadsine),
        (
            "ht_trendline",
            upstream::overlap::ht_trendline(c).expect("upstream ht_trendline"),
        ),
        (
            "ht_trendmode",
            upstream::cycle::ht_trendmode(c)
                .expect("upstream ht_trendmode")
                .into_iter()
                .map(|value| value as f64)
                .collect(),
        ),
    ];

    let mut failures = Vec::new();
    let mut compared = 0usize;
    for (key, actual) in cases {
        match compare_f64_series(key, &actual, &expected(&root, key)) {
            Ok(n) => compared += n,
            Err(err) => failures.push(err),
        }
    }
    let failure_keys = failures
        .iter()
        .map(|failure| failure.split(':').next().expect("failure key"))
        .collect::<Vec<_>>();
    assert_eq!(
        failure_keys,
        ["apo_12_26", "ppo_12_26"],
        "unexpected upstream numeric fixture mismatches after comparing {compared} values:\n{}",
        failures.join("\n")
    );
}

#[test]
fn upstream_matches_our_talib_fixture_for_representative_candle_patterns() {
    let (i, root) = load_fixture();
    let cases: Vec<(&str, Vec<i32>)> = vec![
        (
            "cdldoji",
            upstream::pattern::cdl_doji(&i.open, &i.high, &i.low, &i.close)
                .expect("upstream cdldoji"),
        ),
        (
            "cdlengulfing",
            upstream::pattern::cdl_engulfing(&i.open, &i.high, &i.low, &i.close)
                .expect("upstream cdlengulfing"),
        ),
        (
            "cdlhammer",
            upstream::pattern::cdl_hammer(&i.open, &i.high, &i.low, &i.close)
                .expect("upstream cdlhammer"),
        ),
        (
            "cdlshootingstar",
            upstream::pattern::cdl_shootingstar(&i.open, &i.high, &i.low, &i.close)
                .expect("upstream cdlshootingstar"),
        ),
        (
            "cdl3blackcrows",
            upstream::pattern::cdl_3blackcrows(&i.open, &i.high, &i.low, &i.close)
                .expect("upstream cdl3blackcrows"),
        ),
    ];

    let mut failures = Vec::new();
    let mut compared = 0usize;
    for (key, actual) in cases {
        match compare_i32_series(key, &actual, &expected(&root, key)) {
            Ok(n) => compared += n,
            Err(err) => failures.push(err),
        }
    }
    assert!(
        failures.is_empty(),
        "upstream candle fixture mismatches after comparing {compared} values:\n{}",
        failures.join("\n")
    );
}

fn repeat(values: &[f64], times: usize) -> Vec<f64> {
    let mut out = Vec::with_capacity(values.len() * times);
    for _ in 0..times {
        out.extend_from_slice(values);
    }
    out
}

fn option_sum(values: &[Option<f64>]) -> f64 {
    values.iter().flatten().sum()
}

fn nan_sum(values: &[f64]) -> f64 {
    values.iter().copied().filter(|value| !value.is_nan()).sum()
}

fn time_case(mut f: impl FnMut() -> f64) -> (Duration, f64) {
    let started = Instant::now();
    let checksum = f();
    (started.elapsed(), checksum)
}

#[test]
#[ignore = "prints release-mode performance comparison against upstream talib-rs"]
fn compare_upstream_performance_on_repeated_fixture_data() {
    let (i, _) = load_fixture();
    let repeats = 1024;
    let close = repeat(&i.close, repeats);
    let close2 = repeat(&i.close2, repeats);
    let open = repeat(&i.open, repeats);
    let high = repeat(&i.high, repeats);
    let low = repeat(&i.low, repeats);
    let volume = repeat(&i.volume, repeats);
    let bars = close.len();
    let mut mavp_periods = vec![14.0_f64; bars];
    for period in mavp_periods.iter_mut().skip(bars / 2) {
        *period = 30.0;
    }

    let mut rows = Vec::new();
    macro_rules! bench {
        ($name:literal, $ours:expr, $upstream:expr) => {{
            let (ours_elapsed, ours_sum) = time_case(|| black_box($ours));
            let (up_elapsed, up_sum) = time_case(|| black_box($upstream));
            rows.push((
                $name,
                ours_elapsed,
                up_elapsed,
                ours_elapsed.as_secs_f64() / bars as f64 * 1e9,
                up_elapsed.as_secs_f64() / bars as f64 * 1e9,
                ours_sum,
                up_sum,
            ));
        }};
    }

    bench!(
        "rsi_14",
        option_sum(&ours::rsi(&close, 14)),
        nan_sum(&upstream::momentum::rsi(&close, 14).expect("upstream rsi"))
    );
    bench!(
        "macd_12_26_9",
        {
            let out = ours::macd(&close, 12, 26, 9);
            option_sum(&out.macd) + option_sum(&out.signal) + option_sum(&out.hist)
        },
        {
            let (macd, signal, hist) =
                upstream::momentum::macd(&close, 12, 26, 9).expect("upstream macd");
            nan_sum(&macd) + nan_sum(&signal) + nan_sum(&hist)
        }
    );
    bench!(
        "bbands_20_2",
        {
            let out = ours::bollinger_bands(&close, 20, 2.0);
            option_sum(&out.upper) + option_sum(&out.middle) + option_sum(&out.lower)
        },
        {
            let (upper, middle, lower) =
                upstream::overlap::bbands(&close, 20, 2.0, 2.0, MaType::Sma)
                    .expect("upstream bbands");
            nan_sum(&upper) + nan_sum(&middle) + nan_sum(&lower)
        }
    );
    bench!(
        "correl_30",
        option_sum(&ours::correl(&close, &close2, 30)),
        nan_sum(&upstream::statistic::correl(&close, &close2, 30).expect("upstream correl"))
    );
    bench!(
        "beta_5",
        option_sum(&ours::beta(&close, &close2, 5)),
        nan_sum(&upstream::statistic::beta(&close, &close2, 5).expect("upstream beta"))
    );
    bench!(
        "mavp_14_30",
        option_sum(&ours::mavp(&close, &mavp_periods, 2, 30)),
        nan_sum(
            &upstream::overlap::mavp(&close, &mavp_periods, 2, 30, MaType::Sma)
                .expect("upstream mavp")
        )
    );
    bench!(
        "kama_30",
        option_sum(&ours::kama(&close, 30)),
        nan_sum(&upstream::overlap::kama(&close, 30).expect("upstream kama"))
    );
    bench!(
        "ultosc_7_14_28",
        option_sum(&ours::ultosc(&high, &low, &close, 7, 14, 28)),
        nan_sum(
            &upstream::momentum::ultosc(&high, &low, &close, 7, 14, 28).expect("upstream ultosc")
        )
    );
    bench!(
        "stochrsi_14_5_3",
        {
            let out = ours::stochrsi(&close, 14, 5, 3);
            option_sum(&out.k) + option_sum(&out.d)
        },
        {
            let (k, d) = upstream::momentum::stochrsi(&close, 14, 5, 3, MaType::Sma)
                .expect("upstream stochrsi");
            nan_sum(&k) + nan_sum(&d)
        }
    );
    bench!(
        "linearreg_family_14",
        {
            let out = ours::linear_regression(&close, 14);
            option_sum(&out.line)
                + option_sum(&out.slope)
                + option_sum(&out.angle)
                + option_sum(&out.intercept)
                + option_sum(&out.tsf)
        },
        {
            let line = upstream::statistic::linearreg(&close, 14).expect("upstream linearreg");
            let slope =
                upstream::statistic::linearreg_slope(&close, 14).expect("upstream linearreg_slope");
            let angle =
                upstream::statistic::linearreg_angle(&close, 14).expect("upstream linearreg_angle");
            let intercept = upstream::statistic::linearreg_intercept(&close, 14)
                .expect("upstream linearreg_intercept");
            let tsf = upstream::statistic::tsf(&close, 14).expect("upstream tsf");
            nan_sum(&line) + nan_sum(&slope) + nan_sum(&angle) + nan_sum(&intercept) + nan_sum(&tsf)
        }
    );
    bench!(
        "aroon_14",
        {
            let out = ours::aroon(&high, &low, 14);
            option_sum(&out.up) + option_sum(&out.down) + option_sum(&out.oscillator)
        },
        {
            let (down, up) = upstream::momentum::aroon(&high, &low, 14).expect("upstream aroon");
            let oscillator =
                upstream::momentum::aroon_osc(&high, &low, 14).expect("upstream aroon_osc");
            nan_sum(&up) + nan_sum(&down) + nan_sum(&oscillator)
        }
    );
    bench!(
        "adx_14",
        {
            let out = ours::adx_family(&high, &low, &close, 14);
            option_sum(&out.adx)
                + option_sum(&out.plus_di)
                + option_sum(&out.minus_di)
                + option_sum(&out.dx)
        },
        {
            let adx = upstream::momentum::adx(&high, &low, &close, 14).expect("upstream adx");
            let plus_di =
                upstream::momentum::plus_di(&high, &low, &close, 14).expect("upstream plus_di");
            let minus_di =
                upstream::momentum::minus_di(&high, &low, &close, 14).expect("upstream minus_di");
            let dx = upstream::momentum::dx(&high, &low, &close, 14).expect("upstream dx");
            nan_sum(&adx) + nan_sum(&plus_di) + nan_sum(&minus_di) + nan_sum(&dx)
        }
    );
    bench!(
        "cdl_engulfing",
        ours::cdl_engulfing(&open, &high, &low, &close)
            .iter()
            .map(|value| *value as f64)
            .sum(),
        upstream::pattern::cdl_engulfing(&open, &high, &low, &close)
            .expect("upstream cdl_engulfing")
            .iter()
            .map(|value| *value as f64)
            .sum()
    );
    bench!(
        "adosc_3_10",
        option_sum(&ours::adosc(&high, &low, &close, &volume, 3, 10)),
        nan_sum(
            &upstream::volume::adosc(&high, &low, &close, &volume, 3, 10).expect("upstream adosc")
        )
    );

    println!("bars={bars}");
    println!(
        "case,ours_ns_per_bar,upstream_ns_per_bar,ratio_upstream_over_ours,ours_checksum,upstream_checksum"
    );
    for (name, ours_elapsed, up_elapsed, ours_ns, up_ns, ours_sum, up_sum) in rows {
        println!(
            "{name},{ours_ns:.3},{up_ns:.3},{:.3},{ours_sum:.6},{up_sum:.6}",
            up_elapsed.as_secs_f64() / ours_elapsed.as_secs_f64()
        );
    }
}
