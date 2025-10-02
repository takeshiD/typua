# CI Pipeline Skeleton

GitHub Actions サンプルワークフロー（`ci.yml`）の骨子です。プロジェクトに合わせて分割・調整してください。

```yaml
name: CI

on:
  push:
    branches: ["main", "develop"]
  pull_request:

jobs:
  rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/bin
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
      - name: Fmt
        run: cargo fmt --all -- --check
      - name: Clippy
        run: cargo clippy --all-targets --all-features -- -D warnings
      - name: Tests
        run: cargo test --all --no-fail-fast
```

- `dtolnay/rust-toolchain@stable` で Rust の安定版を取得します。
- `cargo fmt` → `cargo clippy` → `cargo test` の順に実行し、失敗時は早期終了。
- `--no-fail-fast` で全テスト結果を確認可能にしています。
```
