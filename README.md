# Features

- Type check and inference
- Fast
- Low memory


# Type Annotations for type checking

## `LuaCATS` Annotations


## `EmmyLua` Annotations


## What different `LuaCATS` and `EmmyLua`?


# Usage

## Type check

```bash
$ typua check       # type checking current project
$ typua check src/  # type checking specified dir
```

## LSP

```bash
$ typua lsp         # execute lsp current project
$ typua lsp src/    # execute lsp specifed dir
```


# Configuration
typua detects `typua.toml` or `.typua.toml` on your project root.

## Options

| Option                | Type    | Default   | Selectable                                       | Description   |
| --------------------- | ------- | --------- | ------------------------------------------------ | ------------- |
| `enable`              | bool    | `true`    | `true`, `false`                                  |               |
| `syntax`              | enum    | `Lua5.1`  | `Lua5.1`, `Lua5.2`, `Lua5.3`, `Lua5.4`, `LuaJIT` |               |
| `include`             | Path[]  | `[]`      |                                                  |               |
| `exclude`             | Path[]  | `[]`      |                                                  |               |
| `castNumberToInteger` | bool    | `false`   | `true`, `false`                                  |               |
| `weakUnionCheck`      | bool    | `false`   | `true`, `false`                                  |               |
| `weakNilCheck`        | bool    | `false`   | `true`, `false`                                  |               |
| `inferParamType`      | bool    | `false`   | `true`, `false`                                  |               |
| `checkTableShape`     | bool    | `false`   | `true`, `false`                                  |               |
| `inferTableSize`      | integer | 10        |                                                  |               |


```toml
enable = true                   # true | false
syntax = 'Lua5.1'               # "Lua5.1" | "Lua5.2" | "Lua5.3" | "Lua5.4" | "LuaJIT"
include = []                    # List of paths(dirs or files) to be should included in type checking
exclude = []                    # List of paths(dirs or files) to be not should included in type checking
castNumberToInteger = false
weakUnionCheck = false
weakNilCheck = false
inferParamType = false
checkTableShape = false
inferTableSize = 10
```
