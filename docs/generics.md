# Generics Inference Design (typua)

Last updated: 2025-09-20 (UTC)

## Goals
- Rank-1 多相 (Hindley–Milner 相当) を基盤としたジェネリック推論を導入。
- 注釈なしでも一般的なジェネリック関数・テーブルで型引数省略時に推論が成立。
- 注釈あり（---@generic, fun<T> ..., table<K,V>, T[] など）は型環境に反映され、推論と統合。
- LSP(hover/signature help/diagnostic) と CLI(check) の両方で結果を提示。

## Non-Goals (初期段階)
- 多相再帰の自動推論（注釈でのみ許可）。
- 高階の種 (kinds) や型クラスの一般化。
- メタテーブルベースの動的振る舞いの完全解析。

## Terminology
- Type variable (型変数): `'T, 'U, ...`。内部表現は連番 ID。表示時は `T, U, V` に整形。
- Scheme (型スキーム): `∀α. τ`。環境に格納する汎化済み型。
- Substitution (置換): `α ↦ τ` の写像。合成 `S2 ∘ S1` と適用 `S(τ)` を定義。

## Type Language (τ)
Primitives: `nil | any | boolean | number | integer | string | thread | userdata | lightuserdata`。

Constructors:
- Function: `fun(params: Params) -> Returns`
  - `Params = [τ1, τ2, ...] | [τ1, ..., VarArg]`
  - `Returns = [τr1, τr2, ...] | [τr1, ..., VarArg]`
  - `VarArg = τ...` (可変長同種)。
- Tuple: `[τ1, τ2, ...]`（多値戻り・多引数表現に利用）。
- Union: `τ1 | τ2 | ...`（正規化で重複・順序を除去）。
- Optional: `τ?`（糖衣: `τ | nil`）。
- Table (record/map/array):
  - Record: `{ field1: τ1, field2: τ2, ... }` （既定は open 形）。
  - Map: `{ [K]: V }` または `table<K,V>`。
  - Array: `T[]`（糖衣: `{ [integer]: T }`）。
- Type variable: `α`（内部: `Type::Var(TyVarId)`）。

Remarks:
- 既定で table は open record（未宣言フィールド許容）。`---@class (exact)` 注釈で sealed（exact）を明示可能。
- 関数の変性: パラメータは反変、戻り値は共変。table は保守的に不変（Invariant）。

## Environments
Typing env Γ: `Name ↦ Scheme`。Scheme は `Forall {vars: Vec<TyVarId>, body: Type}`。

### Free type variables
- `ftv(τ)`: 型 `τ` 中の自由型変数集合。
- `ftv(σ = ∀α. τ) = ftv(τ) \ α`。
- `ftv(Γ) = ⋃ ftv(σ)` over bindings。

### Generalization (汎化)
`generalize(Γ, τ) = ∀ α. τ` where `α = ftv(τ) \ ftv(Γ)`。

### Instantiation (インスタンス化)
`instantiate(∀ α. τ)` は各 `α` を新鮮な型変数に置換して `τ` を返す。

### Value Restriction (値制限)
HM の健全性のため、汎化は「非拡張的 (non-expansive)」式に限定。
- 初期実装: 関数リテラルとリテラル値のみ自動汎化。
- 強制汎化が必要な場合は `---@generic` でスキームを明示（例: 多相再帰）。

## Constraint Language
制約は推論中に生成され、ソルバで解決する。
- `Eq(τ, τ)`: 単一化制約（同型）。
- `Sub(τ, τ)`: サブタイプ制約（例: `A ≤ A|B`）。
- `HasField(τ_table, name, τ_field)`: レコード/テーブルのフィールド存在と型。
- `Index(τ_table, τ_key, τ_val)`: 添字アクセス（map/array）。
- `Callable(τ_fun, [τ_in...], [τ_out...])`: 呼出可能性と引数/戻り値対応。
- `Narrow(eq|neq|typeis)`: フロー型付けの分岐情報（`x ~= nil`, `type(x) == "string"` 等）。

## Unification and Subtyping
単一化（`unify`）は `Eq` を解決し、置換を生成。occurs check で循環を禁止。

サブタイプ（`≤`）は最小限の規則のみ導入し、完全なサブタイピング系は採らない：
- `A ≤ A|B`、`B ≤ A|B`、`A|B ≤ C` は `A ≤ C ∧ B ≤ C` に還元。
- Optional: `A ≤ A?`、`A? ≤ B?` は `A ≤ B` に還元。
- 関数: `fun P→R ≤ fun P'→R'` は `P' ≤ P ∧ R ≤ R'`。
- Record (open): 要求フィールドは上位互換であれば可（欠損でエラー）。
- Table の変性は不変（Index/HasField 経由で検証）。

Union の単一化は正規化してから集合等式で扱う。`unify(T, A|B)` は
1) `T` が union の場合: 要素ごとに単一化（同数・同順序に正規化）。
2) それ以外: サブタイプ制約 `Sub(T, A|B)` に委譲（分岐の必要があれば `Narrow` で補助）。

## Flow Typing (Narrowing)
CFG 上でブランチ毎に型環境をコピーし、条件に応じて絞り込み：
- `if x ~= nil then` → then ブロックで `x: τ` から `nil` を除去。
- `if type(x) == "string"` → then ブロックで `x: string`、else で `x: τ \ string`。
ブランチ結合点では union をとる（join）。

## Inference Algorithm (概要)
1. 各式/文から制約を生成（bidirectional: 必要に応じ注釈で check モード）。
2. `unify` で `Eq` を解決し置換を蓄積、`Sub` は規則により分解。
3. `Narrow` によるブロック内の一時的環境更新。
4. `let`/`local` 相当の束縛で `generalize`、参照時は `instantiate`。
5. 失敗時は最小反例（最初に矛盾が生じた箇所）を報告。

## Annotation Integration
- `---@generic T, U`（関数/クラス/alias）: スキームの型変数集合を明示。
- `fun<T>(x: T): T` 構文: パラメータ/戻り値に `T` を使用。
- `table<K,V>` / `{ [K]: V }` / `T[]` / `[T1, T2]` / `T?`：既定シンタックスを型へ反映。
- `---@overload fun(...)`：複数署名。適用可能性判定に失敗した場合は曖昧性エラー。
- `---@class (exact) Name`：sealed レコード。未知フィールドは診断。

## Error Messages
- 期待/実際の型を diff で提示（色は LSP/CLI 側に委譲）。
- 型変数は `T, U, V` に命名（循環検出時は `T occurs in ...`）。
- 値制限に触れた場合はヒントを提示（「関数として宣言するか、---@generic で明示」）。

## Examples (可読用サンプル)
```lua
-- identity
---@generic T
---@param x T
---@return T
local function id(x) return x end
local a = id(1)        -- a: integer
local b = id("hi")     -- b: string

-- map
---@generic T, U
---@param f fun(T): U
---@param xs T[]
---@return U[]
local function map(f, xs) ... end

-- optional narrowing
---@param s string?
local function f(s)
  if s ~= nil then
    string.len(s) -- ok: s: string in then-branch
  end
end

-- record field
---@class (exact) P
---@field x number
---@field y number
local p = { x = 1, y = 2 }
-- p.z -> diagnostic (exact)
```

## Implementation Plan (files/modules)
- `src/typing/types.rs`: 型定義（Type, Scheme, Subst, ftv, pretty）。
- `src/typing/unify.rs`: 単一化と `Sub` 規則（occurs, union 正規化）。
- `src/typing/constraints.rs`: 制約定義/ソルバ骨格。
- `src/typing/infer/{expr,stmt}.rs`: 制約生成（式/文）。
- `src/typing/infer/env.rs`: 環境、汎化/インスタンス化、値制限。
- `src/annot/parse.rs`: `---@` 注釈→型 AST 変換。
- `src/diagnostics/types.rs`: エラーメッセージ整形。
- `src/lsp/handlers/{hover,signature,diagnostics}.rs`: LSP 統合。

## Milestones & Acceptance
M1: 型表現/単一化/置換（ユニットテスト通過）
M2: 制約生成(式/関数) + 汎化/インスタンス化（基本例が通る）
M3: 注釈パーサ統合 + LSP hover/signature（簡易プロジェクトで確認）
M4: Flow typing( nil/type checks )
M5: エラーメッセージ/ドキュメント整備

各 M の到達は `cargo test` と `tests/generics_*.rs` のシナリオで確認。

## Risks / Open Questions
- Union を伴う推論の分岐爆発（回避: 正規化と限定的な `Sub` 分解）。
- Lua の再代入/ミュータブル環境と汎化の相互作用（回避: 値制限 + 警告）。
- 多相再帰の扱い（方針: 注釈がある場合のみ許可）。

---

## Formal Specification (詳細)

### Type Grammar (EBNF)
```
Type  := Prim | Var | Fun | Tuple | Union | Opt | Table | Array | Map
Prim  := nil | any | boolean | number | integer | string | thread | userdata | lightuserdata
Var   := 'A' | 'B' | ...   (internal: TyVarId)
Fun   := 'fun' '(' Params ')' ':' Returns
Params:= Types | Types ',' VarArg | /* empty */
Returns:= Types | Types ',' VarArg | /* empty = [] */
VarArg:= Type '...'
Types := Type { ',' Type }
Tuple := '[' Types ']'
Union := Type '|' Type { '|' Type }
Opt   := Type '?'
Table := '{' Fields? '}'
Fields:= Field { ',' Field }
Field := Ident ':' Type | '[' Type ']:' Type
Array := Type '[]'
Map   := 'table' '<' Type ',' Type '>'
```

### Substitution
- 置換 `S: TyVarId → Type`。
- 適用: `S(Prim)=Prim`, `S(Var α)=S(α)` 既定なしは `α`、合成 `(S2 ∘ S1)(τ)=S2(S1(τ))`。
- `ftv` は再帰的に集合を構成（Union/Tuple/Func/Fields を走査）。

### Unification (代表規則)
1) `unify(α, τ)`:
   - if `α == τ` → no-op
   - if `α ∈ ftv(τ)` → occurs error
   - else extend `S` with `α ↦ τ`（ただし `τ` に既存置換を適用してから束縛）
2) `unify(Prim p, Prim q)`:
   - if `p==q` → ok else error
3) `unify(Fun P→R, Fun P'→R')`:
   - 可変長は末尾 `VarArg` を展開し、`|P|` と `|P'|` を整合させて各要素を逆変/共変規則で処理
4) `unify(Tuple [a...], Tuple [b...])`:
   - 要素数が一致し、対応要素を unify
5) `unify(Union U, Union V)`:
   - 正規化（並べ替え・重複除去）後に同長で要素ごと unify（ヒューリスティック: 同型優先）
6) `unify(T, U|V)` or `unify(U|V, T)`:
   - 直接一致が難しい場合は `Sub(T, U|V)` を生成しサブタイプ側へ移譲
7) `unify(Table, Table)`:
   - exact 同士: 同名フィールドを unify、片側に無いフィールドはエラー
   - open を含む: 既知フィールドを unify、未知は許容

エラーには `path`（どの位置の不一致か）を保持する。

### Subtyping (縮約規則)
- `A ≤ A|B`, `B ≤ A|B`。
- `A|B ≤ C` は `A ≤ C ∧ B ≤ C` に分解（正規化でループ防止）。
- Optional: `A ≤ A?`, `A? ≤ B?` ⇒ `A ≤ B`。
- Function: `P' ≤ P ∧ R ≤ R'`。
- Record(open): 要求フィールド `f: T` が表側にも存在し `T_table ≤ T` を満たす。

### Value Restriction (詳細)
- 事前パスでミュータブル集合 `M` を抽出（関数内で LHS に現れるローカル）。
- `let x = e` の汎化条件: `x ∉ M ∧ e` が非拡張（関数/数値/文字列/boolean/nil/テーブルリテラル(要素が非拡張)）。
- 初期イテレーションでは安全側: 「関数・プリミティブ・空テーブル」のみ汎化。拡張は将来タスク。

### Overload Resolution
1) 候補署名ごとに独立した新鮮変数でインスタンス化。
2) 引数→パラメータ単一化、戻り値は期待型（あれば）と整合。
3) 失敗候補は除外。成功が複数ある場合は「より具体的（他を subsume する）型」を選択。
4) 未決着は曖昧性エラー（候補リストを提示）。

### Union 正規化
```
norm(U):
  U1 := 展開・平坦化
  U2 := 同値類ごとに代表元を選択（number vs integer は統一方針に従う）
  U3 := ソート（安定）
  return U3
```

### Pretty Printing
- 変数: `T, U, V, W...`（出現順に割当）。
- 関数: `fun(T1, T2): (R1, R2)`／単一戻りは `: R`。
- Union: `A | B | C`、Optional: `T?` 表示。
- Table: `{ x: A, y: B }`、Map: `{ [K]: V }`、Array: `T[]`。

### Diagnostics (型エラー種別)
- `Mismatch { expected, actual, path }`
- `OccursCheck { var, in_type }`
- `UnknownField { name, in_type }`
- `NotCallable { type }`
- `NoOverloadMatch { callee, candidates, arg_types }`
- `AmbiguousOverload { callee, winners }`
- `ValueRestriction { binding, hint }`

LSP では code と data を付与（例: `typua.mismatch`）。CLI は色付き diff を出力可能（feature flag）。

### LSP 表示仕様
- Hover: 変数/関数/フィールドの最終型を pretty 表示。汎化変数は `∀T.` を前置可能（設定）。
- Signature Help: 選択されたオーバーロードの引数型と現在位置を強調。推論済み型引数を `<T=..., U=...>` の注釈で追記。
- Diagnostics: 上記エラー種別を位置付きで配信。

### 性能・実装方針
- Union/Record は小サイズ前提の SmallVec 最適化。
- 置換は HashMap、パス参照は SmallVec<PathElem>。
- 正規化結果を interning して同型比較を O(1) 近似化（将来最適化）。
- ファイル単位キャッシュ（AST, 注釈, 解析結果）。mtime/内容ハッシュで検証。
- 深さ/時間制限（既定 2000 ステップ or 200ms/ファイル、設定可能）。

### テスト計画
- 単体: `types`, `subst`, `unify`, `union_norm`, `env(instantiate/generalize)`。
- 統合: `infer_call_polymorphic`, `overload_resolution`, `narrow_nil`, `record_exact`。
- 失敗系: `occurs`, `mismatch_path`, `ambiguous_overload`, `value_restriction`。
- Lua サンプル: `fixtures/generics/*.lua` を `cargo test` から読み込み。

### 互換性・注釈準拠
- Lua Language Server の注釈仕様に整合（`---@generic`, `---@class (exact)`, `---@overload`, `table<K,V>`, `T[]`, `T?`）。
- 独自拡張は導入しない（表示の `∀T.` は UI 上の装飾で実装を汚染しない）。
