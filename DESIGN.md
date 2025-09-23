# Annotations

Basicaly, compatibled lua-language-server.

## Basic Types
- `nil`
- `any`
- `boolean`
- `stirng`
- `number`
- `integer`
- `function`
- `table`
- `thread`
- `userdata`
- `lightuserdata`

## Container Type
- Union: `TYPE | TYPE`(Optional syntax `TYPE?`)
- Array: `TYPE[]`
- Tuple: `[TYPE, TYPE]`
- Dictionary: `{[string]: TYPE}`
- Key-Value Table: `table<TYPE, TYPE>`
- Table literal: `{key1: TYPE, key2: TYPE}`
- Function: `fun(PARAM: TYPE[,]): TYPE`

## Type Annotation
- [ ] `---@alias NAME TYPE`: Alias your own type `TYPE` as `NAME`
**Simple Alias**
```lua
---@alias UserID number
```

**Custom Alias**
```lua
---@alias Mode "r" | "w"
```

- [ ] `---[[@as TYPE]]`: Force a type onto expression
- [ ] `---@type TYPE`: Typing expression as `TYPE`

**Value type**
```lua
---@type number
local a = 1
a = "hello"  -- Cannot assign `string` to type `number` [assign-type-mismatch]

---@type number | string
local a = 1
a = "hello"  -- No diagnostic

---@type (string | number)[]
local a = {}
```

**Function type**
- Basic Syntax(using `@param`, `@return`)
```lua
---@param x string
---@param y number
---@return boolean
local f = function(x, y)
    return x == "hello" and y == 42
end
```

- Type Syntax
```lua
---@type fun(string, number): boolean
local f = function(x, y)
    return x == "hello" and y == 42
end
```

```lua

---@type number
local x = "string" -- Cannot assign `string to `number`
```

- [ ] `---@param name[?] Type [desc]`: Typing parameters; `name?` marks optional, `...` captures varargs.
- [ ] `---@class Name[: Parent]`: Declares table/class shapes; combine with `(exact)` for sealed layouts.
**Simple Class**
```lua
---@class Car
local Car = {}
```

**Inherit Class**
```lua
---@class Vehicle
local Vehicle = {}

---@class Car: Vehicle
local Car = {}
```

**Exact Class**
```lua
---@class (exact) Point
---@field x number
---@field y number
local Point = {}
Point.x = 1 -- Ok
Point.y = 2 -- Ok
Point.z = 3 -- Error, Field type mismatch
```

- [ ] `---@cast name Type`: Reinterprets the type of an expression or variable explicitly.
- [ ] `---@async`: Marks asynchronous functions so tools can hint awaited calls.
- [ ] `---@enum Name`: Builds enum-like tables; follow with `---@field VALUE Type` entries.
- [ ] `---@field name Type [desc]` Documents table fields with optional access modifiers.
- [ ] `---@generic T`: Declares type parameters for classes, functions, or aliases.
**Generic Function**
```lua
---@generic T
---@param x T
---@return T
local f = function(x)
    return x * 2
end

--- Type syntax
---@generic T
---@type fun(T): T
local f = function(x)
    return x * 2
end

local x = f(12)         -- Ok, x is infered as number
local y = f("hello")    -- Error, Param type mismatch

---@type boolean
local z = f(12)         -- Error, Assign type mismatch
```
**Generic Class**(Planned)
```lua
---@generic T
---@class Container
---@field _val T
---@field new fun(T): T
---@field set fun(self, T)
---@field get fun(self): Containter
local Container = {}
Containter.__index = Container

---@generic T
---@param value T
---@return Containter
function Containter.new(value)
    local self = setmetatable({}, Containter)
    self._val = value
    return self
end

---@generic T
---@type fun(self, T)
function Container:set(new_val)
    self._val = new_val
end

---@generic T
---@type fun(self): T
function Container:get()
    return self._val
end

local c = Containter.new(12) -- c is inferred as `Container<number>`
c:set("hello") -- Error, Param type mismatch
```

- [ ] `---@meta` Marks the file as a definition/meta file instead of runtime code.
- [ ] `---@module 'name'` Associates the file with a module name used by `require`.
- [ ] `---@nodiscard` Warns when the annotated function's return value is ignored.
- [ ] `---@operator add: fun(self: T, rhs: T): T`: Describes metamethod operator signatures.
- [ ] `---@overload fun(...)`: Adds alternative callable signatures beyond the main declaration.
- [ ] `---@package`: Limits visibility to the current package/module.
- [ ] `---@private`: Restricts visibility to the current file.
- [ ] `---@protected`: Restricts visibility to the class and its subclasses
- [ ] `---@return Type [desc]`: Documents return values; repeat for multiple returns.
- [ ] `---@vararg Type`: Documents varargs (legacy EmmyLua form).

## Misc Annotation
- [ ] `---@diagnostic disable=<id>`: Controls diagnostics with `disable`, `enable`, `push`, `pop`, and optional IDs.
- [ ] `---@deprecated [message]`: Flags symbols as deprecated and shows the message on use.
- [ ] `---@see label` Adds related references or documentation hints.
- [ ] `---@version >=x.y.z`: States the required Lua LS version for the annotation.
- [ ] `---@source file.lua:line`: Records the original source location of a definition.

# Typecheck
## Diagnostics
| Name                   | Category  | Severity | Description |
| ----                   | ----      | ----     | ----        |
| `assign-type-mismatch` | TypeCheck | Error    |             |
| `cast-type-mismatch`   | TypeCheck | Error    |             |
| `param-type-mismatch`  | TypeCheck | Error    |             |
| `field-type-mismatch`  | TypeCheck | Error    |             |
| `return-type-mismatch` | TypeCheck | Error    |             |


# LuaLS型システムの形式的推論規則

## 1. LuaLS型システムの概要と特徴

**アーキテクチャ**：LuaLSはnode-based型表現システムを使用し、各ソース要素を意味的ノードにコンパイルします。型情報は複数の代替案を含むノード構造として表現され、union型を効率的に処理できます。

**設計方針**：
- **構造的型付け**をデフォルトとし、テーブルはshapeで比較
- **フロー感応型分析**による制御フロー内での型の狭化
- **文脈感応推論**で使用パターンから型を推測
- **LuaCATS注釈システム**による開発者主導の型指定

**基本型**：`nil`, `any`, `boolean`, `string`, `number`, `integer`, `function`, `table`, `thread`, `userdata`, `lightuserdata`

## 2. 基本的な型推論規則

### 型環境と型判断

**記法定義**：
```
Γ ⊢ e : τ    型環境Γの下で式eが型τを持つ
Γ ::= ∅ | Γ, x : τ | Γ, α    型環境（変数束縛とtype変数）
τ ::= τ_basic | τ₁ ∪ τ₂ | τ₁ → τ₂ | ∀α.τ | {l₁: τ₁, ...} | τ?
```

### リテラル型規則

**文字列リテラル**：
```
String-Lit:
  ─────────────────
  Γ ⊢ "s" : string
```

**数値リテラル**：
```
Number-Lit:              Integer-Lit:
  ──────────────────       ─────────────────
  Γ ⊢ n.m : number         Γ ⊢ n : integer
```

**ブール・nil リテラル**：
```
Bool-True:               Bool-False:              Nil-Lit:
  ─────────────────       ──────────────────       ──────────────
  Γ ⊢ true : boolean      Γ ⊢ false : boolean      Γ ⊢ nil : nil
```

### 変数参照規則

**変数ルックアップ**：
```
Var:
  x : τ ∈ Γ
  ─────────────
  Γ ⊢ x : τ
```

**変数代入による型推論**：
```
Assign-Infer:
  Γ ⊢ e : τ    x ∉ dom(Γ)
  ─────────────────────────
  Γ, x : τ ⊢ x = e : τ
```

### 関数型規則

**関数定義**：
```
Function-Def:
  Γ, x₁ : τ₁, ..., xₙ : τₙ ⊢ body : τᵣ
  ─────────────────────────────────────────
  Γ ⊢ function(x₁, ..., xₙ) body end : τ₁ × ... × τₙ → τᵣ
```

**関数適用**：
```
Function-App:
  Γ ⊢ f : τ₁ × ... × τₙ → τ    Γ ⊢ e₁ : τ₁    ...    Γ ⊢ eₙ : τₙ
  ─────────────────────────────────────────────────────────────────
  Γ ⊢ f(e₁, ..., eₙ) : τ
```

## 3. LuaCATS型注釈システム

### 注釈による型宣言

**@type注釈**：
```
Type-Annotation:
  Γ ⊢ e : τ'    τ' <: τ    (@type τ)
  ────────────────────────────────────
  Γ ⊢ e : τ
```

**@param注釈**：
```
Param-Annotation:
  (@param x τ)    Γ, x : τ ⊢ body : τᵣ
  ───────────────────────────────────────
  Γ ⊢ function(x, ...) body end : τ → ...
```

**@return注釈**：
```
Return-Annotation:
  Γ ⊢ body : τ'    τ' <: τ    (@return τ)
  ─────────────────────────────────────────
  Γ ⊢ function(...) body end : ... → τ
```

### オプショナル型（?修飾子）

**オプショナル型定義**：
```
Optional-Type:
  τ? ≡ τ ∪ nil
```

**オプショナルパラメータ**：
```
Optional-Param:
  (@param x? τ)
  ──────────────────────────────────────
  Γ, x : τ ∪ nil ⊢ function(x) ... end
```

## 4. サブタイピング規則とUnion/Intersection型

### サブタイピング基本規則

**反射律・推移律**：
```
Sub-Refl:                Sub-Trans:
  ──────────             S <: T    T <: U
  τ <: τ                 ─────────────────
                         S <: U
```

**サブサンプション規則**：
```
Subsumption:
  Γ ⊢ e : S    S <: T
  ─────────────────────
  Γ ⊢ e : T
```

### Union型規則

**Union型導入**：
```
Union-Intro-L:            Union-Intro-R:
  Γ ⊢ e : τ₁               Γ ⊢ e : τ₂
  ─────────────────        ─────────────────
  Γ ⊢ e : τ₁ ∪ τ₂          Γ ⊢ e : τ₁ ∪ τ₂
```

**Union型サブタイピング**：
```
Union-Sub-L:              Union-Sub-R:
  S <: T₁    S <: T₂        T₁ <: S    T₂ <: S
  ────────────────────     ────────────────────
  S <: T₁ ∪ T₂             T₁ ∪ T₂ <: S
```

### フロー感応型狭化

**条件分岐による型狭化**：
```
Flow-Narrow:
  Γ ⊢ x : τ₁ ∪ τ₂    Γ ⊢ cond : boolean    narrows(cond, x, τ₁)
  ───────────────────────────────────────────────────────────────
  Γ, x : τ₁ ⊢ then_branch : τ    Γ, x : τ₂ ⊢ else_branch : τ
```

**nil チェックによる狭化**：
```
Nil-Check:
  Γ ⊢ x : τ?    Γ ⊢ (x ~= nil) : boolean
  ────────────────────────────────────────
  Γ, x : τ ⊢ then_branch : τ'
```

## 5. ジェネリクス型の推論規則

### 汎化規則

**汎化(Generalization)**：
```
Gen:
  Γ ⊢ e : τ    α ∉ FTV(Γ)
  ─────────────────────────
  Γ ⊢ e : ∀α.τ
```

### 特殊化規則

**特殊化(Instantiation)**：
```
Inst:
  Γ ⊢ e : ∀α.τ
  ─────────────────────
  Γ ⊢ e : τ[σ/α]
```

### ジェネリック関数の型推論

**ジェネリック関数定義**：
```
Generic-Fun:
  (@generic T)    Γ, T, x : T ⊢ body : T
  ─────────────────────────────────────────
  Γ ⊢ function(x) body end : ∀T.T → T
```

### 制約付きジェネリクス

**制約ジェネリクス**：
```
Constrained-Generic:
  (@generic T : C)    Γ, T <: C, x : T ⊢ body : τ
  ───────────────────────────────────────────────
  Γ ⊢ function(x) body end : ∀T<:C.T → τ
```

### バッククォート型キャプチャ

**型リテラルキャプチャ**：
```
Type-Capture:
  (@generic T)    (@param class `T`)
  ─────────────────────────────────────────────
  Γ ⊢ function(class) ... end : ∀T.string → T
```

## 6. テーブル型とメタテーブル

### テーブル型定義

**構造的テーブル型**：
```
Table-Type:
  Γ ⊢ e₁ : τ₁    ...    Γ ⊢ eₙ : τₙ
  ─────────────────────────────────────────
  Γ ⊢ {l₁ = e₁, ..., lₙ = eₙ} : {l₁: τ₁, ..., lₙ: τₙ}
```

**テーブルアクセス**：
```
Table-Access:
  Γ ⊢ t : {l₁: τ₁, ..., lᵢ: τᵢ, ..., lₙ: τₙ}
  ───────────────────────────────────────────────
  Γ ⊢ t.lᵢ : τᵢ
```

### 配列型

**配列型定義**：
```
Array-Type:
  (@type τ[])
  ────────────────────────────────
  {[integer]: τ, n: integer, ...}
```

**インデックスアクセス**：
```
Array-Access:
  Γ ⊢ arr : τ[]    Γ ⊢ i : integer
  ──────────────────────────────────
  Γ ⊢ arr[i] : τ
```

### テーブルサブタイピング

**Width Subtyping**：
```
Table-Width-Sub:
  {l₁: τ₁, ..., lₙ: τₙ, lₙ₊₁: τₙ₊₁, ...} <: {l₁: τ₁, ..., lₙ: τₙ}
```

**Depth Subtyping**：
```
Table-Depth-Sub:
  τ₁' <: τ₁    ...    τₙ' <: τₙ
  ────────────────────────────────────
  {l₁: τ₁', ..., lₙ: τₙ'} <: {l₁: τ₁, ..., lₙ: τₙ}
```

### メタメソッド型付け

**演算子オーバーロード**：
```
Metamethod-Add:
  (@operator add(T): U)    Γ ⊢ a : S    Γ ⊢ b : T    S <: self
  ──────────────────────────────────────────────────────────────
  Γ ⊢ a + b : U
```

## 7. Nilableな型の扱い

### Nilableな型定義

**Nullable型コンストラクタ**：
```
Nullable-Def:
  τ? ≡ τ ∪ nil
```

### Nil安全性チェック

**Nil安全アクセス**：
```
Nil-Safe-Access:
  Γ ⊢ obj : τ?    Γ ⊢ (obj ~= nil) : boolean
  ────────────────────────────────────────────
  Γ, obj : τ ⊢ obj.field : τ_field
```

### @cast による Nil 除去

**Nil除去キャスト**：
```
Cast-Remove-Nil:
  Γ ⊢ x : τ?    (@cast x -?)
  ─────────────────────────────
  Γ ⊢ x : τ
```

## 8. 関数オーバーロードと多値返却

### 関数オーバーロード

**オーバーロード定義**：
```
Overload-Def:
  (@overload fun(τ₁): τ₂)    (@overload fun(σ₁, σ₂): σ₃)
  ─────────────────────────────────────────────────────────
  Γ ⊢ f : (τ₁ → τ₂) ∩ (σ₁ × σ₂ → σ₃)
```

**オーバーロード解決**：
```
Overload-Resolution:
  Γ ⊢ f : ∩ᵢ(τᵢ → σᵢ)    Γ ⊢ args : τⱼ    j = resolve(args, {τᵢ})
  ─────────────────────────────────────────────────────────────────
  Γ ⊢ f(args) : σⱼ
```

### 多値返却

**多値返却型**：
```
Multi-Return:
  Γ ⊢ e₁ : τ₁    ...    Γ ⊢ eₙ : τₙ
  ────────────────────────────────────
  Γ ⊢ return e₁, ..., eₙ : τ₁ × ... × τₙ
```

**多値受け取り**：
```
Multi-Assign:
  Γ ⊢ f(...) : τ₁ × ... × τₙ
  ──────────────────────────────────────
  Γ, x₁ : τ₁, ..., xₙ : τₙ ⊢ x₁, ..., xₙ = f(...)
```

## 9. 型の共変性・反変性

### 関数型における変性

**関数サブタイピング（反変・共変）**：
```
Function-Subtyping:
  T₁ <: S₁    S₂ <: T₂
  ──────────────────────
  S₁ → S₂ <: T₁ → T₂
```

### 配列型の変性

**不変配列型**：
```
Array-Invariant:
  Array⟨S⟩ ≮: Array⟨T⟩  (S ≠ T の場合)
```

### テーブル型の変性

**読み取り専用フィールド（共変）**：
```
Table-Covariant:
  S <: T
  ─────────────────
  {readonly field: S} <: {readonly field: T}
```

**書き込み可能フィールド（不変）**：
```
Table-Invariant:
  {field: S} ≮: {field: T}  (S ≠ T の場合)
```

## 10. 実装に基づく推論アルゴリズム

### Node-based型表現

LuaLSは各AST要素を意味的ノードにコンパイルし、型情報を管理します：

```
vm.compileNode: Source → Node
Node = {type: string, alternatives: Type[]}
```

### 制約解決アルゴリズム

**制約生成**：
```
Constraint-Gen:
  Γ ⊢ e : τ  generates  C
  ─────────────────────────
  solve(C) ⊢ τ' where τ' = unify(τ, C)
```

### 増分解析

**増分型チェック**：
```
Incremental-Check:
  Changed(file) → InvalidateCache(deps(file)) → Reanalyze(file ∪ deps(file))
```

### フロー解析エンジン

**制御フロー解析**：
```
Flow-Analysis:
  Γ₀ ⊢ stmt₁ : Γ₁ ⊢ ... ⊢ stmtₙ : Γₙ
  ─────────────────────────────────────
  Γ₀ ⊢ {stmt₁; ...; stmtₙ} : Γₙ
```

