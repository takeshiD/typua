# Lua型検査機 - 詳細データフロー図

## 目次
1. [全体アーキテクチャとデータフロー](#1-全体アーキテクチャとデータフロー)
2. [Salsaベースの増分計算フロー](#2-salsaベースの増分計算フロー)
3. [型チェックの詳細フロー](#3-型チェックの詳細フロー)
4. [型の絞り込み（Type Narrowing）の詳細](#4-型の絞り込みtype-narrowingの詳細)
5. [LSPサーバーのリアルタイム処理フロー](#5-lspサーバーのリアルタイム処理フロー)
6. [永続的データ構造のライフサイクル](#6-永続的データ構造のライフサイクル)
7. [エラー処理とリカバリーフロー](#7-エラー処理とリカバリーフロー)
8. [並列処理とスレッド間通信](#8-並列処理とスレッド間通信)

---

## 1. 全体アーキテクチャとデータフロー

```mermaid
graph TB
    subgraph "外部インターフェース層"
        USER["👤 ユーザー<br/>(エディタでコード編集)"]
        EDITOR["📝 エディタ<br/>(VS Code, Neovim等)"]
        CLI["💻 CLI"]
    end
    
    subgraph "LSPサーバー層"
        LSP_SERVER["🔌 LSP Server<br/>(tower-lsp)<br/>────────<br/>• did_open<br/>• did_change<br/>• hover<br/>• completion"]
        
        HANDLER["📮 Request Handler<br/>────────<br/>リクエストの<br/>ルーティング"]
    end
    
    subgraph "Salsa Database層【増分計算エンジン】"
        DB["🗄️ DatabaseImpl<br/>────────<br/>• キャッシュ管理<br/>• 依存追跡<br/>• 自動無効化"]
        
        INPUT_Q["📥 Input Queries<br/>────────<br/>set_source_text()"]
        
        DERIVED_Q["⚙️ Derived Queries<br/>────────<br/>• parse()<br/>• bind()<br/>• type_check()"]
        
        DEP_GRAPH["📊 依存関係グラフ<br/>────────<br/>Query間の依存を記録"]
        
        CACHE["💾 Query Cache<br/>────────<br/>LRU Eviction"]
    end
    
    subgraph "アプリケーション層"
        PARSER["🔍 Parser<br/>(full_moon)<br/>────────<br/>Lua → AST"]
        
        BINDER["⚙️ Binder<br/>────────<br/>• SymbolTable構築<br/>• CFG構築<br/>• スコープ管理"]
        
        TYPE_CHECKER["🔬 TypeChecker<br/>────────<br/>• 型推論<br/>• Type Narrowing<br/>• エラー診断"]
    end
    
    subgraph "ドメイン層【永続的データ構造】"
        SYMBOL_TABLE["📚 SymbolTable<br/>(im::HashMap)<br/>────────<br/>変数 → 型情報"]
        
        CFG["🔀 ControlFlowGraph<br/>(im::HashMap)<br/>────────<br/>制御フロー表現"]
        
        TYPE_ENV["🌳 TypeEnvironment<br/>(im::HashMap)<br/>────────<br/>実行時型情報"]
        
        TYPE_SYSTEM["🎯 Type System<br/>────────<br/>• Union型<br/>• 型演算<br/>• 絞り込み"]
    end
    
    subgraph "出力"
        DIAGNOSTICS["⚠️ Diagnostics<br/>────────<br/>エラー・警告"]
        HOVER_INFO["ℹ️ Hover Info<br/>────────<br/>型情報"]
        COMPLETIONS["✨ Completions<br/>────────<br/>補完候補"]
    end
    
    %% データフロー
    USER -->|"1. コード編集"| EDITOR
    EDITOR -->|"2. LSP通知"| LSP_SERVER
    CLI -->|"2'. CLI実行"| DB
    
    LSP_SERVER -->|"3. リクエスト"| HANDLER
    HANDLER -->|"4. クエリ実行"| DB
    
    DB <-->|"5. 管理"| INPUT_Q
    DB <-->|"5. 管理"| DERIVED_Q
    DB <-->|"6. 追跡"| DEP_GRAPH
    DB <-->|"7. 参照"| CACHE
    
    INPUT_Q -->|"8. ソース提供"| PARSER
    PARSER -->|"9. AST"| BINDER
    BINDER -->|"10. Bind結果"| TYPE_CHECKER
    
    BINDER -->|"11. 構築"| SYMBOL_TABLE
    BINDER -->|"11. 構築"| CFG
    TYPE_CHECKER -->|"12. 更新"| TYPE_ENV
    TYPE_CHECKER -->|"12. 参照"| TYPE_SYSTEM
    
    TYPE_CHECKER -->|"13. 生成"| DIAGNOSTICS
    TYPE_CHECKER -->|"13. 生成"| HOVER_INFO
    TYPE_CHECKER -->|"13. 生成"| COMPLETIONS
    
    DIAGNOSTICS -->|"14. 返却"| HANDLER
    HOVER_INFO -->|"14. 返却"| HANDLER
    COMPLETIONS -->|"14. 返却"| HANDLER
    
    HANDLER -->|"15. 応答"| LSP_SERVER
    LSP_SERVER -->|"16. 通知"| EDITOR
    EDITOR -->|"17. 表示"| USER
    
    style USER fill:#e1f5ff
    style EDITOR fill:#e1f5ff
    style CLI fill:#e1f5ff
    
    style LSP_SERVER fill:#d4edda
    style HANDLER fill:#d4edda
    
    style DB fill:#fff3cd
    style INPUT_Q fill:#fff3cd
    style DERIVED_Q fill:#fff3cd
    style DEP_GRAPH fill:#fff3cd
    style CACHE fill:#fff3cd
    
    style PARSER fill:#f8d7da
    style BINDER fill:#f8d7da
    style TYPE_CHECKER fill:#f8d7da
    
    style SYMBOL_TABLE fill:#e7e7ff
    style CFG fill:#e7e7ff
    style TYPE_ENV fill:#e7e7ff
    style TYPE_SYSTEM fill:#e7e7ff
    
    style DIAGNOSTICS fill:#d1ecf1
    style HOVER_INFO fill:#d1ecf1
    style COMPLETIONS fill:#d1ecf1
```

---

## 2. Salsaベースの増分計算フロー

```mermaid
graph TB
    subgraph "初回実行（コールドスタート）"
        INIT_INPUT["📝 ユーザー入力<br/>────────<br/>file1.lua:<br/>local x = 10"]
        
        INIT_SET["1️⃣ set_source_text(file1)<br/>────────<br/>Input Queryを設定<br/>⏱️ O(1)"]
        
        INIT_CALL["2️⃣ type_check(db, file1)<br/>────────<br/>Derived Query呼び出し"]
        
        INIT_PARSE["3️⃣ parse(db, file1)<br/>────────<br/>依存記録: type_check → parse<br/>⏱️ 5ms"]
        
        INIT_BIND["4️⃣ bind(db, file1)<br/>────────<br/>依存記録: type_check → bind<br/>⏱️ 15ms"]
        
        INIT_CHECK["5️⃣ 型チェック実行<br/>────────<br/>⏱️ 20ms"]
        
        INIT_CACHE["6️⃣ 結果をキャッシュ<br/>────────<br/>• parse(file1) → AST<br/>• bind(file1) → BindResult<br/>• type_check(file1) → Diagnostics"]
        
        INIT_RESULT["✅ 診断結果<br/>────────<br/>エラー: 0件"]
    end
    
    subgraph "変更検知（ホットパス）"
        CHANGE_INPUT["📝 ファイル変更<br/>────────<br/>file1.lua:<br/>local x = 'hello'"]
        
        CHANGE_SET["1️⃣ set_source_text(file1)<br/>────────<br/>Input Queryを更新<br/>⏱️ O(1)"]
        
        CHANGE_INVALID["2️⃣ 自動無効化<br/>────────<br/>Salsaが依存グラフを走査<br/>────────<br/>❌ parse(file1)<br/>❌ bind(file1)<br/>❌ type_check(file1)<br/>⏱️ 0.1ms"]
        
        CHANGE_CALL["3️⃣ type_check(db, file1)<br/>────────<br/>キャッシュチェック"]
        
        CHANGE_DETECT["4️⃣ キャッシュミス検出<br/>────────<br/>依存が無効化されている"]
        
        CHANGE_REPARSE["5️⃣ parse(db, file1) 再実行<br/>────────<br/>⏱️ 5ms"]
        
        CHANGE_REBIND["6️⃣ bind(db, file1) 再実行<br/>────────<br/>⏱️ 15ms"]
        
        CHANGE_RECHECK["7️⃣ 型チェック再実行<br/>────────<br/>⏱️ 20ms"]
        
        CHANGE_RECACHE["8️⃣ 新しい結果をキャッシュ<br/>────────<br/>古いキャッシュを上書き"]
        
        CHANGE_RESULT["✅ 新しい診断結果<br/>────────<br/>合計: 40ms<br/>（全体再計算なら15000ms）"]
    end
    
    subgraph "依存ファイルへの伝播"
        PROP_FILE2["📄 file2.lua<br/>────────<br/>require('file1')<br/>print(x)"]
        
        PROP_CHECK["1️⃣ type_check(db, file2)<br/>────────<br/>file1への依存あり"]
        
        PROP_LOOKUP["2️⃣ type_check(db, file1)<br/>────────<br/>依存Queryを呼び出し"]
        
        PROP_DETECT["3️⃣ file1の変更を検知<br/>────────<br/>キャッシュが無効"]
        
        PROP_INVALID["4️⃣ file2も無効化<br/>────────<br/>依存チェーンで伝播"]
        
        PROP_RECHECK["5️⃣ file1, file2を再チェック<br/>────────<br/>⏱️ 80ms (2ファイル)"]
        
        PROP_RESULT["✅ 両ファイルの診断<br/>────────<br/>影響範囲を自動特定"]
    end
    
    subgraph "Salsaの内部構造"
        SALSA_INPUT["Input Storage<br/>────────<br/>HashMap<FileId, String>"]
        
        SALSA_DERIVED["Derived Storage<br/>────────<br/>HashMap<QueryKey, Value>"]
        
        SALSA_DEPS["Dependency Graph<br/>────────<br/>HashMap<QueryKey, Vec<Dep>>"]
        
        SALSA_REV["Revision Counter<br/>────────<br/>u64 (変更ごとにインクリ)"]
    end
    
    %% フロー接続
    INIT_INPUT --> INIT_SET
    INIT_SET --> INIT_CALL
    INIT_CALL --> INIT_PARSE
    INIT_PARSE --> INIT_BIND
    INIT_BIND --> INIT_CHECK
    INIT_CHECK --> INIT_CACHE
    INIT_CACHE --> INIT_RESULT
    
    CHANGE_INPUT --> CHANGE_SET
    CHANGE_SET --> CHANGE_INVALID
    CHANGE_INVALID --> CHANGE_CALL
    CHANGE_CALL --> CHANGE_DETECT
    CHANGE_DETECT --> CHANGE_REPARSE
    CHANGE_REPARSE --> CHANGE_REBIND
    CHANGE_REBIND --> CHANGE_RECHECK
    CHANGE_RECHECK --> CHANGE_RECACHE
    CHANGE_RECACHE --> CHANGE_RESULT
    
    PROP_FILE2 --> PROP_CHECK
    PROP_CHECK --> PROP_LOOKUP
    PROP_LOOKUP --> PROP_DETECT
    PROP_DETECT --> PROP_INVALID
    PROP_INVALID --> PROP_RECHECK
    PROP_RECHECK --> PROP_RESULT
    
    %% Salsa内部との関連
    INIT_SET -.->|"書き込み"| SALSA_INPUT
    INIT_CACHE -.->|"書き込み"| SALSA_DERIVED
    INIT_PARSE -.->|"記録"| SALSA_DEPS
    
    CHANGE_SET -.->|"更新"| SALSA_INPUT
    CHANGE_SET -.->|"インクリメント"| SALSA_REV
    CHANGE_INVALID -.->|"参照"| SALSA_DEPS
    
    style INIT_INPUT fill:#e1f5ff
    style CHANGE_INPUT fill:#e1f5ff
    style PROP_FILE2 fill:#e1f5ff
    
    style INIT_SET fill:#d4edda
    style INIT_CALL fill:#d4edda
    style INIT_RESULT fill:#d1ecf1
    
    style CHANGE_SET fill:#fff3cd
    style CHANGE_INVALID fill:#f8d7da
    style CHANGE_DETECT fill:#f8d7da
    style CHANGE_RESULT fill:#d1ecf1
    
    style PROP_CHECK fill:#d4edda
    style PROP_INVALID fill:#f8d7da
    style PROP_RESULT fill:#d1ecf1
    
    style SALSA_INPUT fill:#e7e7ff
    style SALSA_DERIVED fill:#e7e7ff
    style SALSA_DEPS fill:#e7e7ff
    style SALSA_REV fill:#e7e7ff
```

---

## 3. 型チェックの詳細フロー

```mermaid
graph TB
    subgraph "入力"
        SOURCE["📝 Luaソースコード<br/>────────────────<br/>local x: string | nil = get()<br/>if x ~= nil then<br/>  print(x:upper())<br/>else<br/>  print('nil')<br/>end"]
    end
    
    subgraph "Phase 1: パース"
        PARSE_START["1.1 full_moon::parse()<br/>────────────────<br/>⏱️ 5ms"]
        
        PARSE_LEX["1.2 字句解析<br/>────────────────<br/>トークン列生成"]
        
        PARSE_SYNTAX["1.3 構文解析<br/>────────────────<br/>AST構築"]
        
        AST_OUTPUT["✅ AST<br/>────────────────<br/>Block {<br/>  LocalDecl {<br/>    name: 'x',<br/>    type: Union[String, Nil],<br/>    init: Call { ... }<br/>  },<br/>  If {<br/>    condition: BinaryOp { ... },<br/>    then: Block { ... },<br/>    else: Block { ... }<br/>  }<br/>}"]
    end
    
    subgraph "Phase 2: Binder（シンボルテーブル構築）"
        BIND_START["2.1 Binder::new()<br/>────────────────<br/>初期化"]
        
        BIND_SCOPE["2.2 スコープ作成<br/>────────────────<br/>root_scope = ScopeId(0)"]
        
        BIND_LOCAL["2.3 LocalDecl処理<br/>────────────────<br/>変数 'x' を登録<br/>型: Union[String, Nil]"]
        
        BIND_SYMBOL["2.4 SymbolTable更新<br/>────────────────<br/>symbol_table = symbol_table<br/>  .with_symbol('x', Symbol {<br/>    typ: Union[String, Nil],<br/>    scope: ScopeId(0)<br/>  })<br/>────────────────<br/>⚡ 永続的データ構造<br/>元のsymbol_tableは保持"]
        
        BIND_CFG["2.5 CFG構築<br/>────────────────<br/>Entry → LocalDecl<br/>     → Condition<br/>     → Then / Else<br/>     → Merge<br/>     → Exit"]
        
        BIND_OUTPUT["✅ Bind結果<br/>────────────────<br/>• SymbolTable<br/>• ControlFlowGraph<br/>• スコープ情報"]
    end
    
    subgraph "Phase 3: TypeChecker（型チェック）"
        TC_START["3.1 TypeChecker::new()<br/>────────────────<br/>環境初期化"]
        
        TC_ENV["3.2 TypeEnvironment作成<br/>────────────────<br/>env = TypeEnvironment::new()<br/>  .with_binding('x',<br/>    Union[String, Nil])"]
        
        TC_COND["3.3 条件式チェック<br/>────────────────<br/>x ~= nil<br/>────────────────<br/>左辺: Identifier('x')<br/>  → 型を取得: Union[String, Nil]<br/>右辺: Nil<br/>  → 型: Nil<br/>演算子: Ne (~=)"]
        
        TC_NARROW["3.4 絞り込み抽出<br/>────────────────<br/>extract_narrowing()<br/>────────────────<br/>パターンマッチ:<br/>• x ~= nil を検知<br/>• 左辺が変数<br/>• 右辺がnil<br/>────────────────<br/>then絞り込み:<br/>  x → x.exclude_nil()<br/>    = String<br/>────────────────<br/>else絞り込み:<br/>  x → Nil"]
        
        TC_THEN["3.5 then分岐チェック<br/>────────────────<br/>env_then = env<br/>  .with_binding('x', String)<br/>────────────────<br/>check_block(then_branch,<br/>  env_then)<br/>────────────────<br/>print(x:upper())<br/>  x: String ✅<br/>  String.upper exists ✅"]
        
        TC_ELSE["3.6 else分岐チェック<br/>────────────────<br/>env_else = env<br/>  .with_binding('x', Nil)<br/>────────────────<br/>check_block(else_branch,<br/>  env_else)<br/>────────────────<br/>print('nil')<br/>  リテラル文字列 ✅"]
        
        TC_MERGE["3.7 環境マージ<br/>────────────────<br/>merged = env_then.merge(env_else)<br/>────────────────<br/>x: Union[String, Nil]<br/>  (元の型に戻る)"]
        
        TC_OUTPUT["✅ 診断結果<br/>────────────────<br/>diagnostics: []<br/>エラー: 0件<br/>警告: 0件"]
    end
    
    subgraph "永続的データ構造の動作"
        PDS_ORIGINAL["元の環境<br/>────────<br/>env: {<br/>  x: Union[String, Nil]<br/>}<br/>メモリ位置: 0x1000"]
        
        PDS_THEN["then環境<br/>────────<br/>env_then: {<br/>  x: String<br/>}<br/>メモリ位置: 0x1100<br/>────────<br/>⚡ 構造的共有:<br/>変更部分のみコピー"]
        
        PDS_ELSE["else環境<br/>────────<br/>env_else: {<br/>  x: Nil<br/>}<br/>メモリ位置: 0x1200<br/>────────<br/>⚡ 構造的共有:<br/>変更部分のみコピー"]
        
        PDS_MERGE["マージ後環境<br/>────────<br/>merged: {<br/>  x: Union[String, Nil]<br/>}<br/>メモリ位置: 0x1300<br/>────────<br/>⚡ 新しいインスタンス<br/>元の環境は保持"]
    end
    
    %% データフロー
    SOURCE --> PARSE_START
    PARSE_START --> PARSE_LEX
    PARSE_LEX --> PARSE_SYNTAX
    PARSE_SYNTAX --> AST_OUTPUT
    
    AST_OUTPUT --> BIND_START
    BIND_START --> BIND_SCOPE
    BIND_SCOPE --> BIND_LOCAL
    BIND_LOCAL --> BIND_SYMBOL
    BIND_SYMBOL --> BIND_CFG
    BIND_CFG --> BIND_OUTPUT
    
    BIND_OUTPUT --> TC_START
    TC_START --> TC_ENV
    TC_ENV --> TC_COND
    TC_COND --> TC_NARROW
    TC_NARROW --> TC_THEN
    TC_NARROW --> TC_ELSE
    TC_THEN --> TC_MERGE
    TC_ELSE --> TC_MERGE
    TC_MERGE --> TC_OUTPUT
    
    %% 永続的データ構造の関連
    TC_ENV -.->|"作成"| PDS_ORIGINAL
    TC_THEN -.->|"派生"| PDS_THEN
    TC_ELSE -.->|"派生"| PDS_ELSE
    TC_MERGE -.->|"マージ"| PDS_MERGE
    
    PDS_ORIGINAL -.->|"共有"| PDS_THEN
    PDS_ORIGINAL -.->|"共有"| PDS_ELSE
    
    style SOURCE fill:#e1f5ff
    
    style PARSE_START fill:#fff3cd
    style PARSE_LEX fill:#fff3cd
    style PARSE_SYNTAX fill:#fff3cd
    style AST_OUTPUT fill:#d1ecf1
    
    style BIND_START fill:#d4edda
    style BIND_SCOPE fill:#d4edda
    style BIND_LOCAL fill:#d4edda
    style BIND_SYMBOL fill:#d4edda
    style BIND_CFG fill:#d4edda
    style BIND_OUTPUT fill:#d1ecf1
    
    style TC_START fill:#f8d7da
    style TC_ENV fill:#f8d7da
    style TC_COND fill:#f8d7da
    style TC_NARROW fill:#fff3cd
    style TC_THEN fill:#d4edda
    style TC_ELSE fill:#d4edda
    style TC_MERGE fill:#e7e7ff
    style TC_OUTPUT fill:#d1ecf1
    
    style PDS_ORIGINAL fill:#e7e7ff
    style PDS_THEN fill:#e7e7ff
    style PDS_ELSE fill:#e7e7ff
    style PDS_MERGE fill:#e7e7ff
```

---

## 4. 型の絞り込み（Type Narrowing）の詳細

```mermaid
graph TB
    subgraph "絞り込みパターンの判定"
        CONDITION["📋 条件式<br/>────────<br/>x ~= nil"]
        
        PATTERN_MATCH["🔍 パターンマッチング<br/>────────────────<br/>extract_narrowing()"]
        
        PATTERN_1["パターン1<br/>────────<br/>x ~= nil<br/>────────<br/>変数 != nil"]
        
        PATTERN_2["パターン2<br/>────────<br/>x == nil<br/>────────<br/>変数 == nil"]
        
        PATTERN_3["パターン3<br/>────────<br/>type(x) == 'string'<br/>────────<br/>型ガード"]
        
        PATTERN_4["パターン4<br/>────────<br/>not condition<br/>────────<br/>論理否定"]
        
        PATTERN_5["パターン5<br/>────────<br/>x and y<br/>────────<br/>論理積"]
        
        PATTERN_6["パターン6<br/>────────<br/>x or y<br/>────────<br/>論理和"]
    end
    
    subgraph "パターン1の処理: x ~= nil"
        P1_INPUT["入力<br/>────────<br/>変数: x<br/>現在の型:<br/>  Union[String, Nil]"]
        
        P1_ANALYZE["分析<br/>────────<br/>• 左辺: Identifier('x')<br/>• 右辺: Nil<br/>• 演算子: Ne (~=)"]
        
        P1_THEN["then絞り込み<br/>────────<br/>x ~= nil が真<br/>────────<br/>x.exclude_nil()<br/>= String<br/>────────<br/>nilを除外"]
        
        P1_ELSE["else絞り込み<br/>────────<br/>x ~= nil が偽<br/>────────<br/>x == nil<br/>= Nil<br/>────────<br/>nilのみ残す"]
        
        P1_OUTPUT["出力<br/>────────<br/>then: [('x', String)]<br/>else: [('x', Nil)]"]
    end
    
    subgraph "パターン3の処理: type(x) == 'string'"
        P3_INPUT["入力<br/>────────<br/>変数: x<br/>現在の型:<br/>  Union[String, Number]"]
        
        P3_ANALYZE["分析<br/>────────<br/>• Call(type, [x])<br/>• == 'string'"]
        
        P3_THEN["then絞り込み<br/>────────<br/>type(x) == 'string' が真<br/>────────<br/>x.narrow_to(String)<br/>= String<br/>────────<br/>指定型に絞る"]
        
        P3_ELSE["else絞り込み<br/>────────<br/>type(x) != 'string'<br/>────────<br/>Number<br/>────────<br/>他の型"]
        
        P3_OUTPUT["出力<br/>────────<br/>then: [('x', String)]<br/>else: [('x', Number)]"]
    end
    
    subgraph "複合条件の処理"
        COMPLEX_INPUT["複合条件<br/>────────<br/>not (x == nil)"]
        
        COMPLEX_INNER["内側を評価<br/>────────<br/>x == nil<br/>────────<br/>then: [('x', Nil)]<br/>else: [('x', String)]"]
        
        COMPLEX_NOT["not を適用<br/>────────<br/>then/elseを反転"]
        
        COMPLEX_OUTPUT["出力<br/>────────<br/>then: [('x', String)]<br/>else: [('x', Nil)]"]
    end
    
    subgraph "絞り込みの適用"
        APPLY_INPUT["適用前環境<br/>────────<br/>env: {<br/>  x: Union[String, Nil],<br/>  y: Number<br/>}"]
        
        APPLY_NARROW["絞り込み情報<br/>────────<br/>[('x', String)]"]
        
        APPLY_UPDATE["環境更新<br/>────────<br/>env.with_binding('x', String)"]
        
        APPLY_OUTPUT["適用後環境<br/>────────<br/>env_then: {<br/>  x: String,<br/>  y: Number<br/>}"]
    end
    
    subgraph "型演算の詳細"
        TYPE_OP_UNION["Union型作成<br/>────────<br/>Type::union(<br/>  String,<br/>  Number<br/>)<br/>= Union[String, Number]"]
        
        TYPE_OP_EXCLUDE["nil除外<br/>────────<br/>Union[String, Nil]<br/>  .exclude_nil()<br/>= String"]
        
        TYPE_OP_NARROW["特定型に絞る<br/>────────<br/>Union[String, Number]<br/>  .narrow_to(String)<br/>= String"]
        
        TYPE_OP_INCLUDES["nil含有チェック<br/>────────<br/>Union[String, Nil]<br/>  .includes_nil()<br/>= true"]
    end
    
    %% フロー接続
    CONDITION --> PATTERN_MATCH
    
    PATTERN_MATCH --> PATTERN_1
    PATTERN_MATCH --> PATTERN_2
    PATTERN_MATCH --> PATTERN_3
    PATTERN_MATCH --> PATTERN_4
    PATTERN_MATCH --> PATTERN_5
    PATTERN_MATCH --> PATTERN_6
    
    PATTERN_1 --> P1_INPUT
    P1_INPUT --> P1_ANALYZE
    P1_ANALYZE --> P1_THEN
    P1_ANALYZE --> P1_ELSE
    P1_THEN --> P1_OUTPUT
    P1_ELSE --> P1_OUTPUT
    
    PATTERN_3 --> P3_INPUT
    P3_INPUT --> P3_ANALYZE
    P3_ANALYZE --> P3_THEN
    P3_ANALYZE --> P3_ELSE
    P3_THEN --> P3_OUTPUT
    P3_ELSE --> P3_OUTPUT
    
    PATTERN_4 --> COMPLEX_INPUT
    COMPLEX_INPUT --> COMPLEX_INNER
    COMPLEX_INNER --> COMPLEX_NOT
    COMPLEX_NOT --> COMPLEX_OUTPUT
    
    P1_OUTPUT --> APPLY_INPUT
    APPLY_INPUT --> APPLY_NARROW
    APPLY_NARROW --> APPLY_UPDATE
    APPLY_UPDATE --> APPLY_OUTPUT
    
    P1_THEN -.->|"使用"| TYPE_OP_EXCLUDE
    P3_THEN -.->|"使用"| TYPE_OP_NARROW
    P1_ANALYZE -.->|"使用"| TYPE_OP_INCLUDES
    
    style CONDITION fill:#e1f5ff
    style PATTERN_MATCH fill:#fff3cd
    
    style PATTERN_1 fill:#d4edda
    style PATTERN_2 fill:#d4edda
    style PATTERN_3 fill:#d4edda
    style PATTERN_4 fill:#d4edda
    style PATTERN_5 fill:#d4edda
    style PATTERN_6 fill:#d4edda
    
    style P1_INPUT fill:#e1f5ff
    style P1_THEN fill:#d4edda
    style P1_ELSE fill:#f8d7da
    style P1_OUTPUT fill:#d1ecf1
    
    style P3_INPUT fill:#e1f5ff
    style P3_THEN fill:#d4edda
    style P3_ELSE fill:#f8d7da
    style P3_OUTPUT fill:#d1ecf1
    
    style COMPLEX_INPUT fill:#e1f5ff
    style COMPLEX_NOT fill:#fff3cd
    style COMPLEX_OUTPUT fill:#d1ecf1
    
    style APPLY_INPUT fill:#e1f5ff
    style APPLY_UPDATE fill:#fff3cd
    style APPLY_OUTPUT fill:#d1ecf1
    
    style TYPE_OP_UNION fill:#e7e7ff
    style TYPE_OP_EXCLUDE fill:#e7e7ff
    style TYPE_OP_NARROW fill:#e7e7ff
    style TYPE_OP_INCLUDES fill:#e7e7ff
```

---

## 5. LSPサーバーのリアルタイム処理フロー

```mermaid
graph TB
    subgraph "エディタ側"
        USER_EDIT["👤 ユーザー編集<br/>────────<br/>local x = 'hello'"]
        
        EDITOR["📝 エディタ<br/>(VS Code等)<br/>────────<br/>変更を検知"]
    end
    
    subgraph "LSP通信"
        DID_CHANGE["📨 textDocument/didChange<br/>────────────────<br/>JSON-RPC通知<br/>────────────────<br/>{<br/>  'uri': 'file:///test.lua',<br/>  'version': 2,<br/>  'changes': [{<br/>    'range': {...},<br/>    'text': 'hello'<br/>  }]<br/>}"]
        
        REQUEST_HOVER["📨 textDocument/hover<br/>────────────────<br/>JSON-RPC要求<br/>────────────────<br/>{<br/>  'uri': 'file:///test.lua',<br/>  'position': {<br/>    'line': 0,<br/>    'character': 7<br/>  }<br/>}"]
    end
    
    subgraph "LSPサーバー処理"
        LSP_RECEIVE["🔌 通知受信<br/>────────<br/>tower-lsp"]
        
        LSP_PARSE_URI["1️⃣ URI解析<br/>────────<br/>'file:///test.lua'<br/>→ FileId(1)"]
        
        LSP_APPLY["2️⃣ 変更適用<br/>────────<br/>• 範囲特定<br/>• テキスト更新<br/>• バージョン管理"]
        
        LSP_UPDATE_DB["3️⃣ Database更新<br/>────────<br/>db.set_source_text(<br/>  FileId(1),<br/>  new_text<br/>)"]
        
        LSP_DEBOUNCE["4️⃣ デバウンス<br/>────────<br/>500ms待機<br/>────────<br/>連続した変更を<br/>まとめる"]
        
        LSP_TYPECHECK["5️⃣ 型チェック<br/>────────<br/>diagnostics =<br/>  type_check(db, file)"]
        
        LSP_PUBLISH["6️⃣ 診断送信<br/>────────<br/>publishDiagnostics"]
        
        LSP_HOVER["7️⃣ ホバー処理<br/>────────<br/>• 位置→シンボル<br/>• 型情報取得<br/>• マークダウン生成"]
    end
    
    subgraph "Salsa Database"
        SALSA_INVALIDATE["🔄 自動無効化<br/>────────────────<br/>• parse(file) ❌<br/>• bind(file) ❌<br/>• type_check(file) ❌<br/>────────────────<br/>⏱️ 0.1ms"]
        
        SALSA_RECOMPUTE["♻️ 再計算<br/>────────────────<br/>• parse: 5ms<br/>• bind: 15ms<br/>• type_check: 20ms<br/>────────────────<br/>合計: 40ms"]
        
        SALSA_CACHE["💾 キャッシュ更新<br/>────────────────<br/>新しい結果を保存"]
    end
    
    subgraph "応答"
        RESPONSE_DIAG["📤 診断応答<br/>────────────────<br/>{<br/>  'diagnostics': [<br/>    {<br/>      'range': {...},<br/>      'severity': 'Error',<br/>      'message': 'Type error'<br/>    }<br/>  ]<br/>}"]
        
        RESPONSE_HOVER["📤 ホバー応答<br/>────────────────<br/>{<br/>  'contents': {<br/>    'kind': 'markdown',<br/>    'value': '```lua<br/>x: string<br/>```'<br/>  }<br/>}"]
        
        EDITOR_DISPLAY["📝 エディタ表示<br/>────────<br/>• 赤波線<br/>• ホバー情報"]
    end
    
    subgraph "パフォーマンス最適化"
        OPT_INCREMENTAL["📊 増分更新<br/>────────────────<br/>変更ファイルのみ処理<br/>────────────────<br/>1ファイル: 40ms<br/>vs<br/>全体: 15000ms<br/>────────────────<br/>高速化: 375倍"]
        
        OPT_DEBOUNCE["⏱️ デバウンス<br/>────────────────<br/>連続入力中は待機<br/>────────────────<br/>最後の変更のみ処理"]
        
        OPT_ASYNC["⚡ 非同期処理<br/>────────────────<br/>UIをブロックしない<br/>────────────────<br/>tokio::spawn"]
        
        OPT_PARALLEL["🔀 並列処理<br/>────────────────<br/>複数ファイルを<br/>並列で型チェック"]
    end
    
    %% データフロー
    USER_EDIT --> EDITOR
    EDITOR --> DID_CHANGE
    EDITOR --> REQUEST_HOVER
    
    DID_CHANGE --> LSP_RECEIVE
    LSP_RECEIVE --> LSP_PARSE_URI
    LSP_PARSE_URI --> LSP_APPLY
    LSP_APPLY --> LSP_UPDATE_DB
    
    LSP_UPDATE_DB --> SALSA_INVALIDATE
    SALSA_INVALIDATE --> LSP_DEBOUNCE
    LSP_DEBOUNCE --> LSP_TYPECHECK
    
    LSP_TYPECHECK --> SALSA_RECOMPUTE
    SALSA_RECOMPUTE --> SALSA_CACHE
    SALSA_CACHE --> LSP_PUBLISH
    
    REQUEST_HOVER --> LSP_HOVER
    LSP_HOVER --> RESPONSE_HOVER
    
    LSP_PUBLISH --> RESPONSE_DIAG
    RESPONSE_DIAG --> EDITOR_DISPLAY
    RESPONSE_HOVER --> EDITOR_DISPLAY
    
    %% 最適化との関連
    LSP_TYPECHECK -.->|"恩恵"| OPT_INCREMENTAL
    LSP_DEBOUNCE -.->|"実装"| OPT_DEBOUNCE
    LSP_TYPECHECK -.->|"使用"| OPT_ASYNC
    LSP_TYPECHECK -.->|"使用"| OPT_PARALLEL
    
    style USER_EDIT fill:#e1f5ff
    style EDITOR fill:#e1f5ff
    
    style DID_CHANGE fill:#d4edda
    style REQUEST_HOVER fill:#d4edda
    
    style LSP_RECEIVE fill:#fff3cd
    style LSP_PARSE_URI fill:#fff3cd
    style LSP_APPLY fill:#fff3cd
    style LSP_UPDATE_DB fill:#f8d7da
    style LSP_DEBOUNCE fill:#fff3cd
    style LSP_TYPECHECK fill:#f8d7da
    style LSP_PUBLISH fill:#d1ecf1
    style LSP_HOVER fill:#f8d7da
    
    style SALSA_INVALIDATE fill:#f8d7da
    style SALSA_RECOMPUTE fill:#fff3cd
    style SALSA_CACHE fill:#e7e7ff
    
    style RESPONSE_DIAG fill:#d1ecf1
    style RESPONSE_HOVER fill:#d1ecf1
    style EDITOR_DISPLAY fill:#e1f5ff
    
    style OPT_INCREMENTAL fill:#e7e7ff
    style OPT_DEBOUNCE fill:#e7e7ff
    style OPT_ASYNC fill:#e7e7ff
    style OPT_PARALLEL fill:#e7e7ff
```

---

## 6. 永続的データ構造のライフサイクル

```mermaid
graph TB
    subgraph "初期状態"
        INIT_CREATE["🆕 環境作成<br/>────────────────<br/>env0 = TypeEnvironment::new()<br/>────────────────<br/>bindings: {}<br/>────────────────<br/>メモリ: 0x1000<br/>サイズ: 1KB"]
    end
    
    subgraph "変数追加"
        ADD_X["➕ 変数x追加<br/>────────────────<br/>env1 = env0.with_binding(<br/>  'x',<br/>  Union[String, Nil]<br/>)<br/>────────────────<br/>bindings: {<br/>  x: Union[String, Nil]<br/>}<br/>────────────────<br/>メモリ: 0x1100<br/>追加サイズ: +0.1KB"]
        
        ADD_Y["➕ 変数y追加<br/>────────────────<br/>env2 = env1.with_binding(<br/>  'y',<br/>  Number<br/>)<br/>────────────────<br/>bindings: {<br/>  x: Union[String, Nil],<br/>  y: Number<br/>}<br/>────────────────<br/>メモリ: 0x1200<br/>追加サイズ: +0.1KB"]
    end
    
    subgraph "分岐処理"
        BRANCH_THEN["🔀 then分岐<br/>────────────────<br/>env_then = env2.with_binding(<br/>  'x',<br/>  String<br/>)<br/>────────────────<br/>bindings: {<br/>  x: String,<br/>  y: Number<br/>}<br/>────────────────<br/>メモリ: 0x1300<br/>追加サイズ: +0.1KB"]
        
        BRANCH_ELSE["🔀 else分岐<br/>────────────────<br/>env_else = env2.with_binding(<br/>  'x',<br/>  Nil<br/>)<br/>────────────────<br/>bindings: {<br/>  x: Nil,<br/>  y: Number<br/>}<br/>────────────────<br/>メモリ: 0x1400<br/>追加サイズ: +0.1KB"]
    end
    
    subgraph "マージ"
        MERGE["🔗 環境マージ<br/>────────────────<br/>env_merged = <br/>  env_then.merge(env_else)<br/>────────────────<br/>bindings: {<br/>  x: Union[String, Nil],<br/>  y: Number<br/>}<br/>────────────────<br/>メモリ: 0x1500<br/>追加サイズ: +0.2KB"]
    end
    
    subgraph "メモリ構造"
        MEM_SHARED["🧩 共有データ<br/>────────────────<br/>Base HashMap<br/>────────────────<br/>共有されるノード:<br/>• 変更されていない<br/>  エントリ<br/>• ツリー構造の<br/>  一部ノード<br/>────────────────<br/>全バージョンで共有"]
        
        MEM_DIFF1["📦 差分1<br/>────────<br/>env1の差分<br/>────────<br/>+ x: Union[<br/>    String,<br/>    Nil<br/>  ]"]
        
        MEM_DIFF2["📦 差分2<br/>────────<br/>env2の差分<br/>────────<br/>+ y: Number"]
        
        MEM_DIFF3["📦 差分3<br/>────────<br/>env_thenの差分<br/>────────<br/>x: String<br/>(更新)"]
        
        MEM_DIFF4["📦 差分4<br/>────────<br/>env_elseの差分<br/>────────<br/>x: Nil<br/>(更新)"]
    end
    
    subgraph "メモリ効率の比較"
        COMP_MUTABLE["❌ ミュータブル版<br/>────────────────<br/>全体コピー × 5回<br/>────────────────<br/>env0: 1KB<br/>env1: 1KB (clone)<br/>env2: 1KB (clone)<br/>env_then: 1KB (clone)<br/>env_else: 1KB (clone)<br/>env_merged: 1KB (clone)<br/>────────────────<br/>合計: 6KB"]
        
        COMP_IMMUTABLE["✅ イミュータブル版<br/>────────────────<br/>構造的共有<br/>────────────────<br/>Base: 1KB<br/>env1: +0.1KB<br/>env2: +0.1KB<br/>env_then: +0.1KB<br/>env_else: +0.1KB<br/>env_merged: +0.2KB<br/>────────────────<br/>合計: 1.6KB<br/>────────────────<br/>削減率: 73%"]
    end
    
    subgraph "操作のコスト"
        COST_READ["📖 読み取り<br/>────────<br/>env.get('x')<br/>────────<br/>⏱️ O(log n)<br/>────────<br/>HashMapのルックアップ<br/>通常: O(1)<br/>永続的: O(log n)<br/>────────<br/>実質的にはほぼ同じ"]
        
        COST_WRITE["✏️ 書き込み<br/>────────<br/>env.with_binding(<br/>  'x',<br/>  Type<br/>)<br/>────────<br/>⏱️ O(log n)<br/>────────<br/>パスコピー<br/>通常clone: O(n)<br/>永続的: O(log n)<br/>────────<br/>大幅に高速"]
        
        COST_CLONE["📋 コピー<br/>────────<br/>env.clone()<br/>────────<br/>⏱️ O(1)<br/>────────<br/>参照カウント増加<br/>通常clone: O(n)<br/>永続的: O(1)<br/>────────────────<br/>超高速"]
    end
    
    subgraph "ガベージコレクション"
        GC_REF["🔢 参照カウント<br/>────────────────<br/>各バージョンのRC:<br/>────────────────<br/>env0: RC=1<br/>env1: RC=1<br/>env2: RC=1<br/>env_then: RC=0 ← GC対象<br/>env_else: RC=0 ← GC対象<br/>env_merged: RC=1"]
        
        GC_DROP["🗑️ メモリ解放<br/>────────────────<br/>RC=0のバージョンを<br/>自動解放<br/>────────────────<br/>共有データは保持"]
    end
    
    %% データフロー
    INIT_CREATE --> ADD_X
    ADD_X --> ADD_Y
    ADD_Y --> BRANCH_THEN
    ADD_Y --> BRANCH_ELSE
    BRANCH_THEN --> MERGE
    BRANCH_ELSE --> MERGE
    
    %% メモリ構造との関連
    INIT_CREATE -.->|"作成"| MEM_SHARED
    ADD_X -.->|"追加"| MEM_DIFF1
    ADD_Y -.->|"追加"| MEM_DIFF2
    BRANCH_THEN -.->|"追加"| MEM_DIFF3
    BRANCH_ELSE -.->|"追加"| MEM_DIFF4
    
    MEM_SHARED -.->|"共有"| MEM_DIFF1
    MEM_SHARED -.->|"共有"| MEM_DIFF2
    MEM_SHARED -.->|"共有"| MEM_DIFF3
    MEM_SHARED -.->|"共有"| MEM_DIFF4
    
    %% コストとの関連
    ADD_X -.->|"発生"| COST_WRITE
    BRANCH_THEN -.->|"発生"| COST_CLONE
    MERGE -.->|"発生"| COST_READ
    
    %% GCとの関連
    MERGE --> GC_REF
    GC_REF --> GC_DROP
    
    style INIT_CREATE fill:#e1f5ff
    style ADD_X fill:#d4edda
    style ADD_Y fill:#d4edda
    style BRANCH_THEN fill:#d4edda
    style BRANCH_ELSE fill:#f8d7da
    style MERGE fill:#e7e7ff
    
    style MEM_SHARED fill:#fff3cd
    style MEM_DIFF1 fill:#e7e7ff
    style MEM_DIFF2 fill:#e7e7ff
    style MEM_DIFF3 fill:#e7e7ff
    style MEM_DIFF4 fill:#e7e7ff
    
    style COMP_MUTABLE fill:#f8d7da
    style COMP_IMMUTABLE fill:#d4edda
    
    style COST_READ fill:#e7e7ff
    style COST_WRITE fill:#e7e7ff
    style COST_CLONE fill:#e7e7ff
    
    style GC_REF fill:#fff3cd
    style GC_DROP fill:#f8d7da
```

---

## 7. エラー処理とリカバリーフロー

```mermaid
graph TB
    subgraph "正常フロー"
        NORMAL_INPUT["📝 正しいコード<br/>────────<br/>local x: string = 'hello'"]
        
        NORMAL_PARSE["✅ パース成功<br/>────────<br/>AST生成"]
        
        NORMAL_BIND["✅ バインド成功<br/>────────<br/>SymbolTable構築"]
        
        NORMAL_CHECK["✅ 型チェック成功<br/>────────<br/>diagnostics: []"]
        
        NORMAL_OUTPUT["✅ 成功<br/>────────<br/>エラー: 0件"]
    end
    
    subgraph "構文エラー"
        SYNTAX_INPUT["📝 構文エラー<br/>────────<br/>local x: string = "]
        
        SYNTAX_PARSE["❌ パースエラー<br/>────────────────<br/>full_moon::parse()<br/>→ ParseError {<br/>  message: 'Unexpected EOF',<br/>  line: 1,<br/>  column: 18<br/>}"]
        
        SYNTAX_RECOVERY["🔧 エラーリカバリー<br/>────────────────<br/>• 部分的なASTを構築<br/>• エラーノードを挿入<br/>• 後続処理を継続"]
        
        SYNTAX_DIAG["⚠️ 診断生成<br/>────────────────<br/>Diagnostic {<br/>  severity: Error,<br/>  range: (1:18-1:18),<br/>  message: <br/>    'Unexpected end of file'<br/>}"]
        
        SYNTAX_OUTPUT["📤 出力<br/>────────<br/>エラー: 1件<br/>────────<br/>後続処理はスキップ"]
    end
    
    subgraph "型エラー"
        TYPE_INPUT["📝 型エラー<br/>────────<br/>local x: string = 10"]
        
        TYPE_PARSE["✅ パース成功<br/>────────<br/>AST生成"]
        
        TYPE_BIND["✅ バインド成功<br/>────────<br/>SymbolTable構築<br/>────────<br/>x: string<br/>init: Number(10)"]
        
        TYPE_CHECK["❌ 型チェックエラー<br/>────────────────<br/>期待型: string<br/>実際型: number<br/>────────────────<br/>TypeError {<br/>  expected: String,<br/>  actual: Number,<br/>  location: (1:18-1:20)<br/>}"]
        
        TYPE_DIAG["⚠️ 診断生成<br/>────────────────<br/>Diagnostic {<br/>  severity: Error,<br/>  message: <br/>    'Type mismatch: <br/>     expected string, <br/>     got number'<br/>}"]
        
        TYPE_CONTINUE["▶️ 処理継続<br/>────────────────<br/>エラーを記録して<br/>後続のコードも<br/>チェックを続ける"]
        
        TYPE_OUTPUT["📤 出力<br/>────────<br/>エラー: 1件<br/>────────<br/>残りのコードも<br/>チェック済み"]
    end
    
    subgraph "nil参照エラー"
        NIL_INPUT["📝 nil参照<br/>────────<br/>local x: string | nil = get()<br/>print(x:upper())"]
        
        NIL_PARSE["✅ パース成功"]
        
        NIL_BIND["✅ バインド成功<br/>────────<br/>x: Union[String, Nil]"]
        
        NIL_CHECK["❌ nilの可能性<br/>────────────────<br/>x:upper() の呼び出し<br/>────────────────<br/>x の型: <br/>  Union[String, Nil]<br/>────────────────<br/>Nil には upper が<br/>存在しない"]
        
        NIL_DIAG["⚠️ 診断生成<br/>────────────────<br/>Diagnostic {<br/>  severity: Error,<br/>  message: <br/>    'x may be nil. <br/>     Check before use.'<br/>}"]
        
        NIL_SUGGEST["💡 修正提案<br/>────────────────<br/>Suggestion:<br/>'Add nil check:<br/>  if x ~= nil then<br/>    print(x:upper())<br/>  end'"]
        
        NIL_OUTPUT["📤 出力<br/>────────<br/>エラー: 1件<br/>修正提案: 1件"]
    end
    
    subgraph "複数エラーの処理"
        MULTI_INPUT["📝 複数エラー<br/>────────<br/>local x: string = 10<br/>local y: number = 'hello'<br/>print(z)"]
        
        MULTI_COLLECT["📊 エラー収集<br/>────────────────<br/>Vec&lt;Diagnostic&gt;<br/>────────────────<br/>1. Type mismatch (x)<br/>2. Type mismatch (y)<br/>3. Undefined variable (z)"]
        
        MULTI_PRIORITY["🔢 優先順位付け<br/>────────────────<br/>エラーの重要度:<br/>────────────────<br/>1. 構文エラー (最高)<br/>2. 型エラー (高)<br/>3. 未使用変数 (低)"]
        
        MULTI_LIMIT["✂️ 制限<br/>────────────────<br/>エラー数の制限:<br/>────────────────<br/>最大100件まで表示<br/>────────────────<br/>エディタの負荷軽減"]
        
        MULTI_OUTPUT["📤 出力<br/>────────<br/>エラー: 3件<br/>────────<br/>優先順位順に表示"]
    end
    
    subgraph "永続的データ構造とエラー処理"
        PDS_BACKUP["💾 自動バックアップ<br/>────────────────<br/>永続的データ構造の<br/>おかげで、エラー時も<br/>元の状態を保持<br/>────────────────<br/>明示的な復元不要"]
        
        PDS_EXAMPLE["例: if文でエラー<br/>────────────────<br/>if condition then<br/>  error_stmt  ← エラー<br/>else<br/>  valid_stmt<br/>end<br/>────────────────<br/>then分岐のエラーが<br/>else分岐に影響しない"]
        
        PDS_BENEFIT["✅ 利点<br/>────────────────<br/>• エラーの局所化<br/>• 復元処理不要<br/>• デバッグが容易<br/>• 並列処理が安全"]
    end
    
    %% データフロー
    NORMAL_INPUT --> NORMAL_PARSE
    NORMAL_PARSE --> NORMAL_BIND
    NORMAL_BIND --> NORMAL_CHECK
    NORMAL_CHECK --> NORMAL_OUTPUT
    
    SYNTAX_INPUT --> SYNTAX_PARSE
    SYNTAX_PARSE --> SYNTAX_RECOVERY
    SYNTAX_RECOVERY --> SYNTAX_DIAG
    SYNTAX_DIAG --> SYNTAX_OUTPUT
    
    TYPE_INPUT --> TYPE_PARSE
    TYPE_PARSE --> TYPE_BIND
    TYPE_BIND --> TYPE_CHECK
    TYPE_CHECK --> TYPE_DIAG
    TYPE_DIAG --> TYPE_CONTINUE
    TYPE_CONTINUE --> TYPE_OUTPUT
    
    NIL_INPUT --> NIL_PARSE
    NIL_PARSE --> NIL_BIND
    NIL_BIND --> NIL_CHECK
    NIL_CHECK --> NIL_DIAG
    NIL_DIAG --> NIL_SUGGEST
    NIL_SUGGEST --> NIL_OUTPUT
    
    MULTI_INPUT --> MULTI_COLLECT
    MULTI_COLLECT --> MULTI_PRIORITY
    MULTI_PRIORITY --> MULTI_LIMIT
    MULTI_LIMIT --> MULTI_OUTPUT
    
    %% 永続的データ構造との関連
    TYPE_CHECK -.->|"恩恵"| PDS_BACKUP
    NIL_CHECK -.->|"恩恵"| PDS_BACKUP
    PDS_BACKUP --> PDS_EXAMPLE
    PDS_EXAMPLE --> PDS_BENEFIT
    
    style NORMAL_INPUT fill:#e1f5ff
    style NORMAL_PARSE fill:#d4edda
    style NORMAL_BIND fill:#d4edda
    style NORMAL_CHECK fill:#d4edda
    style NORMAL_OUTPUT fill:#d1ecf1
    
    style SYNTAX_INPUT fill:#e1f5ff
    style SYNTAX_PARSE fill:#f8d7da
    style SYNTAX_RECOVERY fill:#fff3cd
    style SYNTAX_DIAG fill:#f8d7da
    style SYNTAX_OUTPUT fill:#f8d7da
    
    style TYPE_INPUT fill:#e1f5ff
    style TYPE_PARSE fill:#d4edda
    style TYPE_BIND fill:#d4edda
    style TYPE_CHECK fill:#f8d7da
    style TYPE_DIAG fill:#f8d7da
    style TYPE_CONTINUE fill:#fff3cd
    style TYPE_OUTPUT fill:#d1ecf1
    
    style NIL_INPUT fill:#e1f5ff
    style NIL_PARSE fill:#d4edda
    style NIL_BIND fill:#d4edda
    style NIL_CHECK fill:#f8d7da
    style NIL_DIAG fill:#f8d7da
    style NIL_SUGGEST fill:#d4edda
    style NIL_OUTPUT fill:#d1ecf1
    
    style MULTI_INPUT fill:#e1f5ff
    style MULTI_COLLECT fill:#fff3cd
    style MULTI_PRIORITY fill:#fff3cd
    style MULTI_LIMIT fill:#fff3cd
    style MULTI_OUTPUT fill:#d1ecf1
    
    style PDS_BACKUP fill:#e7e7ff
    style PDS_EXAMPLE fill:#e7e7ff
    style PDS_BENEFIT fill:#d4edda
```

---

## 8. 並列処理とスレッド間通信

```mermaid
graph TB
    subgraph "メインスレッド"
        MAIN_START["🚀 起動<br/>────────<br/>ワークスペース<br/>1000ファイル"]
        
        MAIN_ENUMERATE["📁 ファイル列挙<br/>────────────────<br/>find_lua_files()<br/>────────────────<br/>file1.lua<br/>file2.lua<br/>...<br/>file1000.lua"]
        
        MAIN_DB["🗄️ Database作成<br/>────────────────<br/>db = DatabaseImpl::new()<br/>────────────────<br/>共有リソース<br/>(thread-safe)"]
        
        MAIN_REGISTER["📝 ファイル登録<br/>────────────────<br/>for file in files {<br/>  db.set_source_text(<br/>    file,<br/>    read_file(file)<br/>  )<br/>}"]
        
        MAIN_PARALLEL["🔀 並列処理開始<br/>────────────────<br/>rayon::par_iter()"]
        
        MAIN_COLLECT["📊 結果収集<br/>────────────────<br/>Vec&lt;Vec&lt;Diagnostic&gt;&gt;<br/>────────────────<br/>全ファイルの診断"]
        
        MAIN_PUBLISH["📤 結果送信<br/>────────────────<br/>LSP Clientへ<br/>publishDiagnostics"]
    end
    
    subgraph "ワーカースレッド1"
        W1_FILE["📄 file1.lua<br/>────────<br/>割り当て"]
        
        W1_CHECK["🔬 型チェック<br/>────────────────<br/>type_check(db, file1)<br/>────────────────<br/>⏱️ 15ms"]
        
        W1_RESULT["✅ 結果<br/>────────<br/>diagnostics1"]
    end
    
    subgraph "ワーカースレッド2"
        W2_FILE["📄 file2.lua<br/>────────<br/>割り当て"]
        
        W2_CHECK["🔬 型チェック<br/>────────────────<br/>type_check(db, file2)<br/>────────────────<br/>⏱️ 20ms"]
        
        W2_RESULT["✅ 結果<br/>────────<br/>diagnostics2"]
    end
    
    subgraph "ワーカースレッド3"
        W3_FILE["📄 file3.lua<br/>────────<br/>割り当て"]
        
        W3_CHECK["🔬 型チェック<br/>────────────────<br/>type_check(db, file3)<br/>────────────────<br/>⏱️ 18ms"]
        
        W3_RESULT["✅ 結果<br/>────────<br/>diagnostics3"]
    end
    
    subgraph "ワーカースレッド4"
        W4_FILE["📄 file4.lua<br/>────────<br/>割り当て"]
        
        W4_CHECK["🔬 型チェック<br/>────────────────<br/>type_check(db, file4)<br/>────────────────<br/>⏱️ 22ms"]
        
        W4_RESULT["✅ 結果<br/>────────<br/>diagnostics4"]
    end
    
    subgraph "スレッドセーフティ"
        SAFETY_DB["🔒 Database<br/>────────────────<br/>• Arc&lt;Mutex&gt; 不要<br/>• &self (共有参照)<br/>• 内部で同期制御<br/>────────────────<br/>Salsaが保証"]
        
        SAFETY_CACHE["💾 共有キャッシュ<br/>────────────────<br/>• 読み取り専用<br/>• RwLock使用<br/>• 複数スレッドから<br/>  同時アクセス可能"]
        
        SAFETY_ENV["🌳 環境<br/>────────────────<br/>• im::HashMap<br/>• 永続的データ構造<br/>• cloneが軽量<br/>• 各スレッドが<br/>  独立したコピーを持つ"]
    end
    
    subgraph "パフォーマンス比較"
        PERF_SINGLE["⏱️ シングルスレッド<br/>────────────────<br/>1000ファイル<br/>────────────────<br/>平均 15ms/file<br/>────────────────<br/>合計: 15000ms<br/>(15秒)"]
        
        PERF_4CORE["⚡ 4コア並列<br/>────────────────<br/>1000ファイル<br/>────────────────<br/>各スレッド 250file<br/>────────────────<br/>合計: 4000ms<br/>(4秒)<br/>────────────────<br/>高速化: 3.75倍"]
        
        PERF_8CORE["⚡⚡ 8コア並列<br/>────────────────<br/>1000ファイル<br/>────────────────<br/>各スレッド 125file<br/>────────────────<br/>合計: 2100ms<br/>(2.1秒)<br/>────────────────<br/>高速化: 7.1倍"]
        
        PERF_16CORE["⚡⚡⚡ 16コア並列<br/>────────────────<br/>1000ファイル<br/>────────────────<br/>各スレッド 62file<br/>────────────────<br/>合計: 1200ms<br/>(1.2秒)<br/>────────────────<br/>高速化: 12.5倍"]
    end
    
    subgraph "依存関係の処理"
        DEP_FILE_A["📄 fileA.lua<br/>────────<br/>require('fileB')"]
        
        DEP_FILE_B["📄 fileB.lua<br/>────────<br/>独立"]
        
        DEP_ORDER["📊 依存順序<br/>────────────────<br/>Salsaが自動解決<br/>────────────────<br/>fileBを先に処理<br/>結果をキャッシュ<br/>fileAがそれを使用"]
        
        DEP_PARALLEL["🔀 並列可能性<br/>────────────────<br/>依存のないファイルは<br/>完全並列<br/>────────────────<br/>依存のあるファイルは<br/>順序制御される"]
    end
    
    subgraph "ロードバランシング"
        LB_WORK_STEAL["🔄 Work Stealing<br/>────────────────<br/>rayonの自動機能<br/>────────────────<br/>速く終わったスレッドが<br/>他のタスクを奪う<br/>────────────────<br/>効率的な負荷分散"]
        
        LB_CHUNK["📦 チャンク分割<br/>────────────────<br/>files.par_chunks(10)<br/>────────────────<br/>小さすぎる: <br/>  オーバーヘッド大<br/>大きすぎる: <br/>  負荷不均衡<br/>────────────────<br/>最適: 10-50ファイル/chunk"]
    end
    
    %% データフロー
    MAIN_START --> MAIN_ENUMERATE
    MAIN_ENUMERATE --> MAIN_DB
    MAIN_DB --> MAIN_REGISTER
    MAIN_REGISTER --> MAIN_PARALLEL
    
    MAIN_PARALLEL --> W1_FILE
    MAIN_PARALLEL --> W2_FILE
    MAIN_PARALLEL --> W3_FILE
    MAIN_PARALLEL --> W4_FILE
    
    W1_FILE --> W1_CHECK
    W1_CHECK --> W1_RESULT
    
    W2_FILE --> W2_CHECK
    W2_CHECK --> W2_RESULT
    
    W3_FILE --> W3_CHECK
    W3_CHECK --> W3_RESULT
    
    W4_FILE --> W4_CHECK
    W4_CHECK --> W4_RESULT
    
    W1_RESULT --> MAIN_COLLECT
    W2_RESULT --> MAIN_COLLECT
    W3_RESULT --> MAIN_COLLECT
    W4_RESULT --> MAIN_COLLECT
    
    MAIN_COLLECT --> MAIN_PUBLISH
    
    %% スレッドセーフティとの関連
    MAIN_DB -.->|"共有"| SAFETY_DB
    W1_CHECK -.->|"参照"| SAFETY_DB
    W2_CHECK -.->|"参照"| SAFETY_DB
    W3_CHECK -.->|"参照"| SAFETY_DB
    W4_CHECK -.->|"参照"| SAFETY_DB
    
    SAFETY_DB -.->|"管理"| SAFETY_CACHE
    W1_CHECK -.->|"使用"| SAFETY_ENV
    
    %% 依存関係
    DEP_FILE_A --> DEP_ORDER
    DEP_FILE_B --> DEP_ORDER
    DEP_ORDER --> DEP_PARALLEL
    
    %% ロードバランシング
    MAIN_PARALLEL -.->|"使用"| LB_WORK_STEAL
    MAIN_PARALLEL -.->|"設定"| LB_CHUNK
    
    style MAIN_START fill:#e1f5ff
    style MAIN_ENUMERATE fill:#d4edda
    style MAIN_DB fill:#fff3cd
    style MAIN_REGISTER fill:#d4edda
    style MAIN_PARALLEL fill:#f8d7da
    style MAIN_COLLECT fill:#e7e7ff
    style MAIN_PUBLISH fill:#d1ecf1
    
    style W1_FILE fill:#d4edda
    style W1_CHECK fill:#d4edda
    style W1_RESULT fill:#d1ecf1
    
    style W2_FILE fill:#d4edda
    style W2_CHECK fill:#d4edda
    style W2_RESULT fill:#d1ecf1
    
    style W3_FILE fill:#d4edda
    style W3_CHECK fill:#d4edda
    style W3_RESULT fill:#d1ecf1
    
    style W4_FILE fill:#d4edda
    style W4_CHECK fill:#d4edda
    style W4_RESULT fill:#d1ecf1
    
    style SAFETY_DB fill:#e7e7ff
    style SAFETY_CACHE fill:#e7e7ff
    style SAFETY_ENV fill:#e7e7ff
    
    style PERF_SINGLE fill:#f8d7da
    style PERF_4CORE fill:#fff3cd
    style PERF_8CORE fill:#d4edda
    style PERF_16CORE fill:#d4edda
    
    style DEP_FILE_A fill:#e1f5ff
    style DEP_FILE_B fill:#e1f5ff
    style DEP_ORDER fill:#fff3cd
    style DEP_PARALLEL fill:#e7e7ff
    
    style LB_WORK_STEAL fill:#e7e7ff
    style LB_CHUNK fill:#e7e7ff
```

---

## 補足説明

### 図の読み方

- **色分け**
  - 🔵 青系: 入力・ユーザー操作
  - 🟢 緑系: 正常処理・成功
  - 🟡 黄色系: 中間処理・計算中
  - 🔴 赤系: エラー・異常系
  - 🟣 紫系: データ構造・内部状態

- **矢印の種類**
  - 実線 (→): データフロー
  - 点線 (-.->): 参照・依存関係
  
- **処理時間の表記**
  - ⏱️ マーク: 実行時間の目安

### 主要な技術ポイント

1. **Salsa**: 増分計算を自動化し、変更時の再計算を最小化
2. **永続的データ構造**: メモリ効率的な状態管理と自動バックアップ
3. **LSP**: エディタとのリアルタイム連携
4. **並列処理**: 複数ファイルの同時処理で高速化

これらのフロー図により、Lua型検査機の動作を詳細に理解できます。
