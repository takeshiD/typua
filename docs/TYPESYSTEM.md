# Lua型システムの推論規則

## 1. 型システムの概要と特徴

**設計方針**：
- **構造的型付け**をデフォルトとし、テーブルはshapeで比較
- **フロー感応型分析**による制御フロー内での型の狭化
- **文脈感応推論**で使用パターンから型を推測
- **LuaCATS注釈システム**による開発者主導の型指定

**基本型**
- `nil`
- `any`
- `boolean`
- `string`
- `number`
- `integer`
- `function`
- `table`
- `thread`
- `userdata`
- `lightuserdata`

**for Subtyping**
- `unknown`: top-type
- `never`: bottom-type

## 2. 基本的な型推論規則

### 型環境と型判断

**記法定義**：

$$
\Gamma \vdash e : \tau
$$

型環境 $\Gamma$ の下で式 $e$ が型 $\tau$ を持つことを表す。

$$
\Gamma ::= \varnothing \mid \Gamma, x : \tau \mid \Gamma, \alpha
$$

型環境（変数束縛と型変数）。

$$
\tau ::= \tau_{\text{basic}} \mid \tau_1 \cup \tau_2 \mid \tau_1 \to \tau_2 \mid \forall \alpha.\tau \mid \{ l_1 : \tau_1, \ldots \} \mid \tau?
$$


### リテラル型規則

**文字列リテラル**：

$$
\text{String-Lit}\quad\dfrac{}{\Gamma \vdash \text{"s"} : \text{string}}
$$

**数値リテラル**：

$$
\text{Number-Lit}\quad\dfrac{}{\Gamma \vdash n.m : \text{number}}\qquad
\text{Integer-Lit}\quad\dfrac{}{\Gamma \vdash n : \text{integer}}
$$

**ブール・nil リテラル**：

$$
\text{Bool-True}\quad\dfrac{}{\Gamma \vdash \text{true} : \text{boolean}}\qquad
\text{Bool-False}\quad\dfrac{}{\Gamma \vdash \text{false} : \text{boolean}}\qquad
\text{Nil-Lit}\quad\dfrac{}{\Gamma \vdash \text{nil} : \text{nil}}
$$

### 変数参照規則

**変数ルックアップ**：

$$
\text{Var}\quad\dfrac{x : \tau \in \Gamma}{\Gamma \vdash x : \tau}
$$

**変数代入による型推論**：

$$
\text{Assign-Infer}\quad\dfrac{\Gamma \vdash e : \tau \quad x \notin \operatorname{dom}(\Gamma)}{\Gamma, x : \tau \vdash x = e : \tau}
$$

### 関数型規則

**関数定義**：

$$
\text{Function-Def}\quad\dfrac{\Gamma, x_1 : \tau_1, \ldots, x_n : \tau_n \vdash \text{body} : \tau_r}{\Gamma \vdash \text{function}(x_1, \ldots, x_n)\;\text{body}\;\text{end} : \tau_1 \times \cdots \times \tau_n \to \tau_r}
$$

**関数適用**：

$$
\text{Function-App}\quad\dfrac{\Gamma \vdash f : \tau_1 \times \cdots \times \tau_n \to \tau \quad \Gamma \vdash e_1 : \tau_1 \quad \cdots \quad \Gamma \vdash e_n : \tau_n}{\Gamma \vdash f(e_1, \ldots, e_n) : \tau}
$$

## 3. LuaCATS型注釈システム

### 注釈による型宣言

**@type注釈**：

$$
\text{Type-Annotation}\quad\dfrac{\Gamma \vdash e : \tau' \quad \tau' <: \tau \quad (@\text{type}\;\tau)}{\Gamma \vdash e : \tau}
$$

**@param注釈**：

$$
\text{Param-Annotation}\quad\dfrac{(@\text{param}\;x\;\tau) \quad \Gamma, x : \tau \vdash \text{body} : \tau_r}{\Gamma \vdash \text{function}(x, \ldots)\;\text{body}\;\text{end} : \tau \to \cdots}
$$

**@return注釈**：

$$
\text{Return-Annotation}\quad\dfrac{\Gamma \vdash \text{body} : \tau' \quad \tau' <: \tau \quad (@\text{return}\;\tau)}{\Gamma \vdash \text{function}(\ldots)\;\text{body}\;\text{end} : \cdots \to \tau}
$$

### オプショナル型（?修飾子）

**オプショナル型定義**：

$$
\text{Optional-Type}\quad \tau? \equiv \tau \cup \text{nil}
$$

**オプショナルパラメータ**：

$$
\text{Optional-Param}\quad\dfrac{(@\text{param}\;x?\;\tau)}{\Gamma, x : \tau \cup \text{nil} \vdash \text{function}(x)\;\ldots\;\text{end}}
$$

### 注釈の無い場合の型付け

**注釈の無い変数束縛**：

$$
\text{Var-Init-Unknown}\quad\dfrac{\Gamma \vdash e : \tau' \quad \tau' <: \tau \quad (@\text{type}\;\tau)}{\Gamma \vdash e : \tau}
$$

**@param注釈**：

$$
\text{Param-Annotation}\quad\dfrac{(@\text{param}\;x\;\tau) \quad \Gamma, x : \tau \vdash \text{body} : \tau_r}{\Gamma \vdash \text{function}(x, \ldots)\;\text{body}\;\text{end} : \tau \to \cdots}
$$

**@return注釈**：

$$
\text{Return-Annotation}\quad\dfrac{\Gamma \vdash \text{body} : \tau' \quad \tau' <: \tau \quad (@\text{return}\;\tau)}{\Gamma \vdash \text{function}(\ldots)\;\text{body}\;\text{end} : \cdots \to \tau}
$$


## 4. 部分型付け規則とUnion/Intersection型

### 部分型付け基本規則

**反射律・推移律**：

$$
\text{Sub-Refl}\quad\dfrac{}{\tau <: \tau}\qquad
\text{Sub-Trans}\quad\dfrac{S <: T \quad T <: U}{S <: U}
$$

**包摂規則**：
$$
\text{Subsumption}\quad\dfrac{\Gamma \vdash e : S \quad S <: T}{\Gamma \vdash e : T}
$$

### Union型規則

**Union型導入**：

$$
\text{Union-Intro-L}\quad\dfrac{\Gamma \vdash e : \tau_1}{\Gamma \vdash e : \tau_1 \cup \tau_2}\qquad
\text{Union-Intro-R}\quad\dfrac{\Gamma \vdash e : \tau_2}{\Gamma \vdash e : \tau_1 \cup \tau_2}
$$

**Union型部分型付け**：

$$
\text{Union-Sub-L}\quad\dfrac{S <: T_1 \quad S <: T_2}{S <: T_1 \cup T_2}\qquad
\text{Union-Sub-R}\quad\dfrac{T_1 <: S \quad T_2 <: S}{T_1 \cup T_2 <: S}
$$

### フロー感応型狭化

**条件分岐による型狭化**：

$$
\text{Flow-Narrow}\quad\dfrac{\Gamma \vdash x : \tau_1 \cup \tau_2 \quad \Gamma \vdash \text{cond} : \text{boolean} \quad \text{narrows}(\text{cond}, x, \tau_1)}{\Gamma, x : \tau_1 \vdash \text{then\_branch} : \tau \quad \Gamma, x : \tau_2 \vdash \text{else\_branch} : \tau}
$$

**nil チェックによる狭化**：

$$
\text{Nil-Check}\quad\dfrac{\Gamma \vdash x : \tau? \quad \Gamma \vdash (x \neq \text{nil}) : \text{boolean}}{\Gamma, x : \tau \vdash \text{then\_branch} : \tau'}
$$

## 5. ジェネリクス型の推論規則

### 汎化規則

**汎化(Generalization)**

$$
\text{Gen}\quad\dfrac{\Gamma \vdash e : \tau \quad \alpha \notin \operatorname{FTV}(\Gamma)}{\Gamma \vdash e : \forall \alpha.\tau}
$$

### 特殊化規則

**特殊化(Instantiation)**

$$
\text{Inst}\quad\dfrac{\Gamma \vdash e : \forall \alpha.\tau}{\Gamma \vdash e : \tau[\sigma/\alpha]}
$$

### ジェネリック関数の型推論

**ジェネリック関数定義**：

$$
\text{Generic-Fun}\quad\dfrac{(@\text{generic}\;T) \quad \Gamma, T, x : T \vdash \text{body} : T}{\Gamma \vdash \text{function}(x)\;\text{body}\;\text{end} : \forall T.\,T \to T}
$$

### 制約付きジェネリクス

**制約ジェネリクス**：

$$
\text{Constrained-Generic}\quad\dfrac{(@\text{generic}\;T : C) \quad \Gamma, T <: C, x : T \vdash \text{body} : \tau}{\Gamma \vdash \text{function}(x)\;\text{body}\;\text{end} : \forall T<:C.\, T \to \tau}
$$

### バッククォート型キャプチャ

**型リテラルキャプチャ**：

$$
\text{Type-Capture}\quad\dfrac{(@\text{generic}\;T) \quad (@\text{param}\;\text{class}\;`T`)}{\Gamma \vdash \text{function}(\text{class})\;\ldots\;\text{end} : \forall T.\,\text{string} \to T}
$$

## 6. テーブル型とメタテーブル

### テーブル型定義

**構造的テーブル型**：

$$
\text{Table-Type}\quad\dfrac{\Gamma \vdash e_1 : \tau_1 \quad \cdots \quad \Gamma \vdash e_n : \tau_n}{\Gamma \vdash \{l_1 = e_1, \ldots, l_n = e_n\} : \{l_1 : \tau_1, \ldots, l_n : \tau_n\}}
$$

**テーブルアクセス**：

$$
\text{Table-Access}\quad\dfrac{\Gamma \vdash t : \{l_1 : \tau_1, \ldots, l_i : \tau_i, \ldots, l_n : \tau_n\}}{\Gamma \vdash t.l_i : \tau_i}
$$

### 配列型

**配列型定義**：

$$
\text{Array-Type}\quad (@\text{type}\;\tau[]) \Rightarrow \{[\text{integer}] : \tau,\; n : \text{integer},\; \ldots\}
$$

**インデックスアクセス**：

$$
\text{Array-Access}\quad\dfrac{\Gamma \vdash \text{arr} : \tau[] \quad \Gamma \vdash i : \text{integer}}{\Gamma \vdash \text{arr}[i] : \tau}
$$

### テーブルサブタイピング

**Width Subtyping**：

$$
\text{Table-Width-Sub}\quad \{l_1 : \tau_1, \ldots, l_n : \tau_n, l_{n+1} : \tau_{n+1}, \ldots\} <: \{l_1 : \tau_1, \ldots, l_n : \tau_n\}
$$

**Depth Subtyping**：

$$
\text{Table-Depth-Sub}\quad\dfrac{\tau_1' <: \tau_1 \quad \cdots \quad \tau_n' <: \tau_n}{\{l_1 : \tau_1', \ldots, l_n : \tau_n'\} <: \{l_1 : \tau_1, \ldots, l_n : \tau_n\}}
$$

### メタメソッド型付け

**演算子オーバーロード**：

$$
\text{Metamethod-Add}\quad\dfrac{(@\text{operator}\;\text{add}(T) : U) \quad \Gamma \vdash a : S \quad \Gamma \vdash b : T \quad S <: \text{self}}{\Gamma \vdash a + b : U}
$$

## 7. Nilableな型の扱い

### Nilableな型定義

**Nullable型コンストラクタ**：

$$
\text{Nullable-Def}\quad \tau? \equiv \tau \cup \text{nil}
$$

### Nil安全性チェック

**Nil安全アクセス**：

$$
\text{Nil-Safe-Access}\quad\dfrac{\Gamma \vdash \text{obj} : \tau? \quad \Gamma \vdash (\text{obj} \neq \text{nil}) : \text{boolean}}{\Gamma, \text{obj} : \tau \vdash \text{obj.field} : \tau_{\text{field}}}
$$

### @cast による Nil 除去

**Nil除去キャスト**：

$$
\text{Cast-Remove-Nil}\quad\dfrac{\Gamma \vdash x : \tau? \quad (@\text{cast}\;x\;-?)}{\Gamma \vdash x : \tau}
$$

## 8. 関数オーバーロードと多値返却

### 関数オーバーロード

**オーバーロード定義**：

$$
\text{Overload-Def}\quad\dfrac{(@\text{overload}\;\text{fun}(\tau_1) : \tau_2) \quad (@\text{overload}\;\text{fun}(\sigma_1, \sigma_2) : \sigma_3)}{\Gamma \vdash f : (\tau_1 \to \tau_2) \cap (\sigma_1 \times \sigma_2 \to \sigma_3)}
$$

**オーバーロード解決**：

$$
\text{Overload-Resolution}\quad\dfrac{\Gamma \vdash f : \bigcap_i (\tau_i \to \sigma_i) \quad \Gamma \vdash \text{args} : \tau_j \quad j = \text{resolve}(\text{args}, \{\tau_i\})}{\Gamma \vdash f(\text{args}) : \sigma_j}
$$

### 多値返却

**多値返却型**：

$$
\text{Multi-Return}\quad\dfrac{\Gamma \vdash e_1 : \tau_1 \quad \cdots \quad \Gamma \vdash e_n : \tau_n}{\Gamma \vdash \text{return}\; e_1, \ldots, e_n : \tau_1 \times \cdots \times \tau_n}
$$

**多値受け取り**：

$$
\text{Multi-Assign}\quad\dfrac{\Gamma \vdash f(\ldots) : \tau_1 \times \cdots \times \tau_n}{\Gamma, x_1 : \tau_1, \ldots, x_n : \tau_n \vdash x_1, \ldots, x_n = f(\ldots)}
$$

## 9. 型の共変性・反変性

### 関数型における変性

**関数サブタイピング（反変・共変）**：

$$
\text{Function-Subtyping}\quad\dfrac{T_1 <: S_1 \quad S_2 <: T_2}{S_1 \to S_2 <: T_1 \to T_2}
$$

### 配列型の変性

**不変配列型**：

$$
\text{Array-Invariant}\quad S \neq T \Longrightarrow \neg\bigl(\text{Array}\langle S\rangle <: \text{Array}\langle T\rangle\bigr)
$$

### テーブル型の変性

**読み取り専用フィールド（共変）**：

$$
\text{Table-Covariant}\quad\dfrac{S <: T}{\{\text{readonly field} : S\} <: \{\text{readonly field} : T\}}
$$

**書き込み可能フィールド（不変）**：

$$
\text{Table-Invariant}\quad S \neq T \Longrightarrow \neg\bigl(\{\text{field} : S\} <: \{\text{field} : T\}\bigr)
$$

## 10. 実装に基づく推論アルゴリズム

### Node-based型表現

LuaLSは各AST要素を意味的ノードにコンパイルし、型情報を管理します：

$$
vm.compileNode : \text{Source} \to \text{Node}
$$

$$
\text{Node} = \{\text{type} : \text{string},\; \text{alternatives} : \text{Type}[]\}
$$

### 制約解決アルゴリズム

**制約生成**：

$$
\text{Constraint-Gen}\quad\dfrac{\Gamma \vdash e : \tau \quad \text{generates } C}{\text{solve}(C) \vdash \tau' \quad \text{where } \tau' = \text{unify}(\tau, C)}
$$

### 増分解析

**増分型チェック**：

$$
\text{Incremental-Check}\quad \text{Changed}(\text{file}) \to \text{InvalidateCache}(\text{deps}(\text{file})) \to \text{Reanalyze}(\text{file} \cup \text{deps}(\text{file}))
$$

### フロー解析エンジン

**制御フロー解析**：

$$
\text{Flow-Analysis}\quad\dfrac{\Gamma_0 \vdash \text{stmt}_1 : \Gamma_1 \vdash \cdots \vdash \text{stmt}_n : \Gamma_n}{\Gamma_0 \vdash \{\text{stmt}_1; \ldots; \text{stmt}_n\} : \Gamma_n}
$$


# Architecture
```text
  +-------------------------------------------+
  | typua バイナリ (src/main.rs)              |
  | - run() が CLI で受けた Command を dispatch|
  +-----------------------+-------------------+
                          |
                          v
  +-----------------------+---------------------------+
  | CLI ファサード (src/cli/mod.rs)                  |
  | - clap で引数解析                                |
  | - Config 読込 → CheckOptions / LspOptions        |
  +-----------+-----------+--------------------------+
              |           |
              |           v
              |   +-------+--------------------------------------------+
              |   | LSP サーバ (src/lsp.rs)                           |
              |   | - tower-lsp / tokio                               |
              |   | - DocumentState に text と TypeInfo map を保持    |
              |   +----+---------------------+------------------------+
              |        |                     |
              |        | collect_workspace_registry()                  |
              |        |  └─ workspace::collect_source_files()         |
              |        v                     v
              |   +----+---------------------+---------------------------------------------+
              |   | タイプチェッカー基盤 (src/typechecker)                                 |
              |   | - annotation::AnnotationIndex::from_{ast,source}                       |
              |   | - typed_ast::build_typed_ast()                                         |
              |   |     * TypedAST: ノードに局所注釈とclass_hintsを保持する拡張ASTビュー   |
              |   | - checker::TypeChecker::check_program()                                |
              |   | - types::{CheckReport, TypeRegistry, TypeInfo…}                        |
              |   |     * TypeRegistry: ワークスペース横断でクラス/enum定義を集約する表     |
              |   +----+---------------------+---------------------------------------------+
              |        ^
              |        |
              v        |
  +-----------+--------+---------------------+
  | handle_check() (src/main.rs:25)          |
  | └─ checker::run(CheckOptions)            |
  |    - workspace::collect_source_files()   |
  |    - full_moon 解析 → CheckResult        |
  +-----------+-----------------------------+
              |
              v
  +-----------+-----------------------------+
  | Diagnostics (src/diagnostics.rs)        |
  | - Severity, Diagnostic, Display 実装    |
  +-----------------------------------------+

```

# Typecheck level
## Values and Types
### Boolean
## Arithmetic operator

## Bitwise Operator(Lua5.2 or newer and LuaJIT)

## Relational Operator

## Control Structure
### if statement
- `false` and `nil` are treated as `false`
- all values different from `false` and `nil` are treated as `true`

# Edge Cases
## Behavior about only annotation bool and logic operator `and` `or`
`and`と`or`は短絡評価のため
```lua
a and b => a がfalsy(nil / false)の場合a, truthy(nil/false以外)の場合はbを返す
a or b => a がfalsy(nil / false)の場合b, truthy(nil/false以外)の場合はaを返す
```

`and`と`or`演算子の型検査を行なう場合次のケースが発生する
```lua
---@type boolean
local x
local y = x and 12
```
つまりxは宣言されているがassignされておらず、型宣言だけされており、その状態でyの評価に利用するケースである。

lua-lsの場合は次のようになる(inlay hint likeに表示を付加した)
```lua
---@type boolean
local x: boolean
local y: integer|false = x and 12
```

この場合次の考え方があるだろう

1. xはassignされていないので、nilがassignされていると相当しているとみなす
つまりand
```lua
---@type boolean
local x: boolean
local y: nil = x and 12
```
2. xはassignされているが、仮の

