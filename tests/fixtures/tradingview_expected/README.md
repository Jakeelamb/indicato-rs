# TradingView Expected CSV Fixtures

This folder contains committed TradingView chart-export CSVs for the
`ta_indicators::tradingview` golden tests.

Do not place Rust-generated output here. These files are useful only when the
values came from TradingView or a Pine-compatible runtime. The Rust test harness
compares the declared indicator columns in each CSV against the corresponding
Rust implementation.

Run the gate with:

```bash
cargo test --test tradingview_golden
```

The checked-in `ohlcv.csv` is the 180-row Sabertooth-derived backtest fixture
used for the current TradingView golden exports, ordered oldest to newest. It
must include case-insensitive `open`, `high`, `low`, `close`, and `volume`
columns. Extra columns such as time and symbol are ignored.

The harness looks for one CSV per golden validation family:

- `volume_liquidity_sweep.csv`
- `volumetric_trend_ribbon.csv`
- `momentum_vol_composite.csv`
- `adaptive_baseline.csv`
- `self_strength_oscillator.csv`
- `mariashi_renko_system.csv`
- `anchored_vwap.csv`
- `wyckoff_phase.csv`
- `darvas_turtle_breakout.csv`
- `ichimoku_cloud_state.csv`
- `heikin_ashi_transform.csv`

Expected indicator CSVs may include extra TradingView export columns such as
time or OHLCV fields. The harness ignores extras and compares only declared
indicator columns for that family. Each expected indicator CSV must include at
least one declared non-`bar_index` indicator column, otherwise there is nothing
meaningful to compare.

Quoted CSV cells, escaped double quotes, and commas inside quoted values are
supported. Numeric cells may include thousands separators; the comparator
removes commas only for numeric tolerance checks.

Missing exports must be listed in `pending.tsv` with a real external blocker.
