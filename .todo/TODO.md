# TODO: typua Implementation Status

進捗と残タスクを機能単位で追跡します（Conventional Commits準拠でPR/コミットに反映）。

## Parser / Annotation
- [x] `---@type` 解析（単一、`?`、`|`、配列 `T[]`）
- [x] 関数型 `fun(param: T): R` 解析（vararg `T...`）
- [x] ジェネリック適用 `base<Arg,...>` の解析（例: `table<string, number>`）
- [x] 連想テーブル `{ [K]: V }` の解析
- [x] タプル `[A, B]` の解析（暫定: `Applied(tuple, ...)` として表現）
- [x] full_moonベースの注釈抽出API `AnnotationIndex::from_ast(&Ast, &str)` 追加
- [x] 旧 `from_source(&str)` は暫定互換として残しつつ利用箇所を `from_ast` へ移行
- [x] `---@param` / `---@field` のコメント文字列を抽出して保持
- [x] `---@` 直前の空白（例: `--- @class`）や途中のコメント行を許容する柔軟なパーサ調整
- [ ] 辞書・タプルの厳密化（専用`TypeKind`の追加検討）
- [x] `---@alias` の型エイリアス解析と `TypeRegistry` への登録（再解決可能な形で）
- [x] `---@generic` 宣言・適用の解析（関数/クラス）と型推論への反映
- [ ] `---@overload` や `---@cast` 等 LuaLS 拡張アノテーションの解析と checker 連携

## TypedAST
- [x] `typed_ast` モジュール新設（AST定義、変換器）
- [x] 主要ステートメントの変換（`LocalAssign`, `Assign`, `Function`, `Return`）
- [x] 代表的な式の変換（数値・文字列・テーブル・二項演算・変数・フィールド・関数呼び出し）
- [x] `AnnotationIndex` を用いた注釈の付与（パラメータ・return の集約）
- [x] 変換の単体テスト（最小ケース）
- [ ] full_moon AST 全網羅の変換
- [ ] Luau/CfxLua固有バリアント（`CompoundAssignment`, `IfExpression`, `InterpolatedString`, `TypeAssertion` 等）の取扱方針を決定（実装or未対応診断）
- [x] Lua 5.2+/LuaJIT 構文（`Goto`, `Label`）を `Stmt` へ追加し、TypedAST で扱う
- [ ] 位置情報（`TextRange`）の精度向上（全ノード）
- [x] `token_range` が取得できない場合のフォールバック（`Option<TextRange>` など）を導入し `(0,0)` へ落ちないようにする（暫定: 無効範囲を避けるガード追加）
- [x] `merge_ranges` で無効範囲を検出し子ノード範囲へ置換する仕組みを追加
- [x] `Identifier` 生成時に `TokenReference::basic_symbol` 起因の前後空白を補正し、Luau特有トークンでも範囲がずれないようにする

## Type Check Pipeline
- [x] `check_ast_with_registry` で TypedAST を生成（パイプライン挿入）
- [x] アノテーション抽出を `from_ast` ベースに切替
- [x] 意味解析/型解析の実処理を TypedAST ベースへ移行
- [x] 旧チェック処理からの段階的な置き換え（アダプタ）
- [x] 配列型の inlay hint で完全な型表記（例: `boolean[]`, `(boolean|number)[]`）を表示
- [x] 配列型の推論を `table` ではなく配列要素型として扱い、診断メッセージに要素型ミスマッチを反映
- [ ] `CheckResult` に TypedAST 由来の情報を付加（設計）
- [x] 多値返り値 (`---@return` 複数) のチェックと診断メッセージ整備
- [x] 関数型 (`fun(...)`) の注釈/推論サポートと型文字列表現の整備

## LSP/CLI/Config
- [ ] CLI: `--config/-c` オプション実装（引数優先・相対/絶対パス対応）と単体テスト
  - [ ] `.typua.toml` と LSP 設定の相互運用テスト
  - [x] `Config::load_from_dir` の単体テスト追加（基本読込ケース）
- [ ] Config 適用: `runtime.version`/`runtime.include`/`workspace.library` を CLI / LSP / checker のファイル解決に反映
  - [x] `workspace.library` を CLI / LSP / checker に取り込み、追加ライブラリから型定義を収集
  - [x] `workspace.ignore_dir` / `workspace.use_gitignore` を利用して探索対象から除外するロジックを追加
- [ ] LSP: `LspOptions.root` を初期化で活用し、URI→Path 変換を正規化
- [ ] LSP: `.typua.toml`/`workspace.library` を監視・再読込し、外部ライブラリからの型取得を安定化
- [ ] LSP キャパビリティと診断位置の検証（hover/signature help）

## Testing
- [x] `annotation` のテスト拡充（関数型/辞書/タプル/ジェネリック適用）
- [x] `typed_ast` の変換テスト（スモーク）
- [x] `from_ast` の挙動を間接検証（既存checkerテストが全通過）
- [x] ワークスペース横断のTypedAST生成・参照テスト
- [x] 負例テスト（不正アノテーションや壊れた型式）
- [ ] TypedAST 化した checker の新規ユニットテスト（narrowing/演算検証の追加ケース）
- [ ] Luau/Lua52系構文を含むフィクスチャでTypedASTカバレッジと診断位置の検証を追加
- [x] 多値返り値の正/異常系テスト（単体・ワークスペース）
- [x] 関数型推論の単体テスト（戻り値・クラスフィールド）
- [x] 多値返り値Luaフィクスチャのパターン検証テスト

## Tooling
- [x] `cargo clippy --all-targets --all-features` の警告ゼロ
- [x] `cargo fmt --all` の整形確認
- [ ] CI（ビルド・テスト・fmt・clippy）
  - [x] `docs/ci.md` に GitHub Actions 骨子を追加

## リスク/メモ
- TypedAST 切替は段階的に実施。現状は生成のみで拘束。
- `TypeKind` の拡張は互換性に注意。
- `from_ast` はトークン/コメント依存のためfull_moonのTrivia仕様変更に注意。

- [x] `from_ast` の本実装（full_moonトークン/トリビアから `---@` コメントを収集し直後ステートメント行に割当）
  - [x] 空行や通常コメントを跨いだ連続ブロックの扱い（現行互換）
  - [x] 先頭や孤立の `---@class`/`---@enum`/`---@field` の登録（TypeRegistry）
  - [x] パフォーマンス検討（1ファイル内の一回走査・キャッシュ）
- [x] `from_ast` 専用の単体テストを追加
  - [x] 直前ブロックと割当の境界（空行あり/なし、複数行）
  - [x] ステートメントなしのヘッダファイル（クラス定義のみ）
  - [x] 末尾コメントが次行コードに紐付かないことの確認
- [x] 関数アノテーション（`---@param` / `---@return`）がTypedASTと型検査双方に反映される統合テストを追加
- [ ] `from_source` の段階的廃止または`cfg(feature)`化
- [ ] TypedAST の網羅拡張（If/While/For/Call引数 等）
- [ ] 意味/型解析をTypedASTベースに移行（アダプタ→完全移行）
- [ ] LSP: hover/signature helpで注釈/型情報を参照
- [ ] ドキュメント更新（README/AGENTS.md: 解析フロー・制約）
