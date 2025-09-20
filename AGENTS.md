# Lua Typechecker
このプロジェクトはLua言語の型チェッカーとして構築します。

以下は型チェッカーの仕様と実装に際しての諸注意を記載します。

# Type Annotations
- Lua Language Server annotations use `---@` comments and support primitives, arrays, tuples, functions, unions, generics, and optional types.citeturn1search0turn3open3

## Canonical Type Names
`nil`, `any`, `boolean`, `string`, `number`, `integer`, `function`, `table`, `thread`, `userdata`, `lightuserdata`.citeturn3open3

## Type Forms
- Arrays: `Type[]`; dictionaries: `{ [KeyType]: ValueType }`; generics: `table<KeyType, ValueType>`; tuples: `[TypeA, TypeB]`; optionals: `Type?`; varargs: `Type...`; function signatures: `fun(param: Type): Return`.citeturn3open3

## Annotation Reference (Lua Language Server)
| Annotation                                  | Purpose / Key Syntax                                                            |
| ---                                         | ---                                                                             |
| `---@async`                                 | Marks asynchronous functions so tools can hint awaited calls.                   |
| `---@cast name Type`                        | Reinterprets the type of an expression or variable explicitly.                  |
| `---@class Name[: Parent]`                  | Declares table/class shapes; combine with `(exact)` for sealed layouts.         |
| `---@diagnostic disable=<id>`               | Controls diagnostics with `disable`, `enable`, `push`, `pop`, and optional IDs. |
| `---@deprecated [message]`                  | Flags symbols as deprecated and shows the message on use.                       |
| `---@enum Name`                             | Builds enum-like tables; follow with `---@field VALUE Type` entries.            |
| `---@field name Type [desc]`                | Documents table fields with optional access modifiers.                          |
| `---@generic T`                             | Declares type parameters for classes, functions, or aliases.                    |
| `---@meta`                                  | Marks the file as a definition/meta file instead of runtime code.               |
| `---@module 'name'`                         | Associates the file with a module name used by `require`.                       |
| `---@nodiscard`                             | Warns when the annotated function's return value is ignored.                    |
| `---@operator add: fun(self: T, rhs: T): T` | Describes metamethod operator signatures.                                       |
| `---@overload fun(...)`                     | Adds alternative callable signatures beyond the main declaration.               |
| `---@package`                               | Limits visibility to the current package/module.                                |
| `---@param name Type [desc]`                | Documents parameters; `name?` marks optional, `...` captures varargs.           |
| `---@private`                               | Restricts visibility to the current file.                                       |
| `---@protected`                             | Restricts visibility to the class and its subclasses.                           |
| `---@return Type [desc]`                    | Documents return values; repeat for multiple returns.                           |
| `---@see label`                             | Adds related references or documentation hints.                                 |
| `---@source file.lua:line`                  | Records the original source location of a definition.                           |
| `---@type Type`                             | Assigns a type to locals or globals.                                            |
| `---@vararg Type`                           | Documents varargs (legacy EmmyLua form).                                        |
| `---@version >=x.y.z`                       | States the required Lua LS version for the annotation.                          |
Annotation reference derived from the Lua Language Server annotations guide.citeturn1search0turn3open0turn3open1turn3open2

## Additional Helpers
- Use `--[[@as Type]]` or `--[=[@as Type]=]` to coerce the inferred type of an expression inline.citeturn3open1
- Build literal enumerations with chained `---| 'value'` lines or `---@alias Name 'v1'|'v2'`; prefer `---@enum` for tables with documented fields.citeturn3open1
- Scope enforcement comes from combining `---@class (exact)` with `---@private`, `---@protected`, or `---@package`.citeturn3open2
- Annotations support Markdown formatting (headings, bold, code blocks) and paragraph breaks using `---`.citeturn3open0

## Example
```lua
---@enum Mode
---@field Immediate '"immediate"'
---@field Deferred '"deferred"'

---@class (exact) Job
---@field run fun(mode: Mode): boolean

---@async
---@param job Job
---@param retries integer?
---@return boolean handled
---@nodiscard
local function dispatch(job, retries)
    local attempts = retries or 1
    ---@cast attempts integer
    return job.run('"immediate"') and attempts > 0
end
```


# LanguageServer Capabilities
- diagnostic
- hover
- inlay hints
- completion
- find reference
- goto type definition
- rename
- signature help

# Implementation Guidelines
実装に際して以下の機能に対して対応するcrateをcargo addして実装をしてください。

| 機能                             | crate             |
| ------                           | -------           |
| Lua構文解析器                    | full_moon         |
| LSP Server                       | tower-lsp         |
| コマンドラインオプションパーサー | clap              |
| エラーハンドリング               | anyhow, thiserror |
| 非同期処理ランタイム             | tokio             |
| アサーション                     | pretty_assertions |

## CLI Options
コマンド名は`typua`とします。

### CLIとしての利用
```bash
$ typua check {path}
```
指定したpath以下のluaスクリプトの型チェックを行う。

実行した同ディレクトリ内に設定ファイル`.typua.toml`があれば読み込んで適用する。

### LSPサーバーとして起動
```bash
$ typua lsp
```
現在のディレクトリ内のluaスクリプトの型チェックを行う。

実行した同ディレクトリ内に設定ファイル`.typua.toml`があれば読み込んで適用する。

## Config File
`.typua.toml`という名前がデフォルトで読み込まれることとする。

```toml
[runtime]
version = "luajit" # lua51 | lua52 | lua53 | lua54
include = [
    "*.lua",
    "**/init.lua",
    "~/.luarocks/share/lua/5.3/*.lua",      # expand `~` as user home
    "$HOME/.luarocks/share/lua/5.3/*.lua",  # expand environment var
    "/usr/share/5.3/*.lua",
    "/usr/share/lua/5.3/*/init.lua"
]

[workspace]
library = [
    "/path/to/nvim/runtime/lua"
]
```

## LSP Config
`.typua.toml`はプロジェクト毎の設定ファイルとして機能するが、neovimなどのエディタでLSP起動設定をする場合は以下のように設定する。

### neovim builtin-lsp
```lua
require('lspconfig').typua_ls.setup{
    settings = {
        Lua = {
            runtime = {
                version = 'lua53',
                include = {
                    "*.lua",
                    "**/init.lua",
                    "~/.luarocks/share/lua/5.3/*.lua",      -- expand `~` as user home
                    "$HOME/.luarocks/share/lua/5.3/*.lua",  -- expand environment var
                    "/usr/share/5.3/*.lua",
                    "/usr/share/lua/5.3/*/init.lua"
                }
            },
            workspace = {
                library = {
                    "/path/to/nvim/runtime/lua"
                }
            }
        }
    }
}
```

# Repository Guidelines

## Build, Test, and Development Commands
- `cargo build` — compile the project and catch compile-time regressions early.
- `cargo run [-- <args>]` — build then run the binary locally.
- `cargo test` — run unit and integration suites; append `-- --ignored` for slow paths.
- `cargo fmt --all` — enforce Rustfmt defaults locally or with `-- --check` in CI.
- `cargo clippy --all-targets --all-features` — lint for common mistakes before review.

## Coding Style & Naming Conventions
- Let `rustfmt` handle layout (4-space indent, trailing commas, grouped imports) and keep modules focused.
- Prefer `Result`/`Option`; guard invariants with `debug_assert!` when warranted.
- Follow idiomatic casing (`snake_case`, `CamelCase`, `SCREAMING_SNAKE_CASE`) and capture safety or panic notes in `///` doc comments for new public APIs.


## Testing Guidelines
- Keep fast unit tests near the source and summarize end-to-end scenarios in `tests/`.
- Name tests `fn target_scenario_expected()` and ensure `cargo test`, `cargo fmt -- --check`, and `cargo clippy` succeed before review.

## Git Commit & Pull Request Guidelines
- Use Conventional Commits (`feat:`, `fix:`, `chore:`) with imperative summaries and flag breaking changes.
- Reference issue IDs, list manual verification steps, and keep PR scope focused.

