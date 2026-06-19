# ta-indicators

Warmup-exact Rust port of [TA-Lib](https://ta-lib.org/) batch indicators. Outputs use
`Option<f64>` with `None` on warmup bars, matching TA-Lib's emitted-value semantics bar
for bar (not just post-warmup tails).

Designed as a standalone library: zero runtime dependencies, deterministic parity tests
against committed TA-Lib reference fixtures.

The crates.io package is `ta-indicators`; the Rust library import name is `ta_indicators`.

## Status

- Publish target: crates.io package `ta-indicators`, Rust crate import `ta_indicators`.
- Runtime dependencies: none.
- License: MIT plus BSD-3-Clause notice for adapted rolling-window techniques.
- Parity gate: 121 checked-in TA-Lib reference series.
- Package dry run: 15 files, about 254 KiB compressed.

## Install

```toml
[dependencies]
ta-indicators = "0.1"
```

```rust
use ta_indicators::{cdl_engulfing, ht_dcperiod, macd, rsi};

let rsi_14 = rsi(&closes, 14);
let macd_out = macd(&closes, 12, 26, 9);
let patterns = cdl_engulfing(&opens, &highs, &lows, &closes);
let dominant_cycle = ht_dcperiod(&closes);
```

## Layout

```
src/lib.rs       Overlap, momentum, volatility, volume, stats, Hilbert transforms
src/candles.rs   Shared candle-settings framework + 61 CDL pattern functions
tests/           Warmup-exact parity harness (121 fixture keys, two test files)
scripts/         Fixture regeneration via Python TA-Lib (dev-only)
```

### `src/lib.rs`

Single-crate API for non-pattern indicators: moving averages, MACD family, RSI,
Bollinger bands, ADX family, Aroon, SAR/SAREXT, linear regression, BETA/CORREL,
Hilbert stack (`ht_*`, `mama`), and Sabertooth-adjacent helpers (`price_context`,
`bop`, etc.).

Multi-output functions return small structs (`Macd`, `AdxFamily`, `BollingerBands`, …)
or tuples (`ht_phasor`, `ht_sine`, `mama`).

### `src/candles.rs`

TA-Lib-compatible candle pattern recognition:

- `CandleSetting` / `RangeType` — shared body/shadow thresholds (TA-Lib defaults).
- `Candles` — OHLCV wrapper with `real_body`, shadows, `color`, `range`, `average`.
- `cdl_*` — 61 pattern detectors returning `Vec<i32>` (`0`, `±100`, `±200`).

## Validation

| File | Keys | Lookback groups |
| --- | ---: | --- |
| `tests/talib_parity.rs` | 111 | Momentum, overlap, stats, SAR, CDL, … |
| `tests/talib_parity_ht.rs` | 10 | Hilbert / MAMA family |

Policy: **warmup-exact** — Rust must match TA-Lib wherever the fixture has a non-null
reference value, including the warmup region.

Regenerate fixtures (requires Python TA-Lib):

```bash
python3 -m venv artifacts/venv-talib
artifacts/venv-talib/bin/pip install TA-Lib numpy
artifacts/venv-talib/bin/python scripts/gen_parity_fixtures.py
cargo test
```

Release gate:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo test
cargo publish --dry-run
```

Performance comparison against `talib-rs`:

```bash
cargo test --release --test upstream_talib_compare \
  compare_upstream_performance_on_repeated_fixture_data -- --ignored --nocapture
```

## Performance

The release perf probe expands the parity fixture to 409,600 bars and compares
`ta-indicators` against `talib-rs 0.1.2` with default features disabled. Ratio is
`upstream / ta-indicators`, so values above `1.0x` mean this crate is faster.
Numbers below are from the publication-prep run on the local release build.

| Case | Ratio |
| --- | ---: |
| `linearreg_family_14` | 1.612x |
| `mavp_14_30` | 1.538x |
| `bbands_20_2` | 1.524x |
| `aroon_14` | 1.234x |
| `adx_14` | 1.221x |
| `kama_30` | 1.126x |
| `rsi_14` | 1.079x |
| `adosc_3_10` | 1.055x |

Known slower cases in the same probe:

| Case | Ratio | Note |
| --- | ---: | --- |
| `correl_30` | 0.936x | Rolling-state path is close, but this local run still trails upstream slightly. |
| `macd_12_26_9` | 0.833x | TA-Lib warmup alignment differs from upstream checksum, so this path is kept parity-first. |
| `stochrsi_14_5_3` | 0.796x | Rolling min/max path is correct, but not consistently faster on this fixture. |
| `ultosc_7_14_28` | 0.750x | Rolling BP/TR sums avoid rescans, but upstream remains faster here. |
| `cdl_engulfing` | 0.739x | Kept TA-Lib fixture-exact endpoint behavior instead of the faster upstream shortcut. |
| `beta_5` | 0.618x | Rolling-state fast path is correct, but still trails upstream on this fixture. |

Performance work is intentionally conservative: optimized paths preserve TA-Lib
fixture parity and fall back to scan-based logic where non-finite data would change
behavior.

## Coverage

The checked-in parity suite currently covers 121 TA-Lib reference series:

- 50 numeric indicator outputs across overlap, momentum, volatility, volume,
  price transforms, statistics, SAR/SAREXT, MACD/MACDEXT-SMA, and linear regression.
- 10 Hilbert/MAMA outputs.
- 61 `CDL*` candle pattern outputs.

Remaining gaps are mostly non-SMA `matype` variants, generic binary operators,
and combined min/max exports.

## Scope

- Batch API only; this is not a streaming indicator engine.
- Pure Rust implementation; Python TA-Lib is used only to regenerate parity fixtures.
- Candle patterns use TA-Lib's default candle settings and return TA-Lib-style
  integer signals (`0`, `±100`, `±200`).

## Publication Checklist

Before publishing:

```bash
cargo fmt --all -- --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo publish --dry-run
```

Secret scan used for release preparation:

```bash
rg -n "(AKIA[0-9A-Z]{16}|gh[pousr]_[A-Za-z0-9_]{20,}|github_pat_[A-Za-z0-9_]+|sk-[A-Za-z0-9_-]{20,}|xox[baprs]-[A-Za-z0-9-]+|BEGIN (RSA|DSA|EC|OPENSSH|PGP|PRIVATE) KEY|api[_-]?key\s*[:=]|token\s*[:=]|password\s*[:=]|secret\s*[:=])" -S . --glob '!target/**' --glob '!.git/**'
```

The crate does not require credentials, tokens, network services, or runtime
configuration.

## License

MIT and BSD-3-Clause. See `LICENSE` and `THIRD_PARTY_NOTICES.md`.
