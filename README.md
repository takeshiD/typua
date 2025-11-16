<h1 align="center">Typua</h1>

`typua` is a lua typechecker for Lua5.1, 5.2, 5.3, 5.4, LuaJIT.

Compatibled type-annotation syntax [lua-language-server](https://github.com/luals/lua-language-server).


# Features
- 🚀 Blazing Fast Typecheck
- 💾 Low memory usage
- 🖥️ Language Server Support
- 🌕️ Lua5.1, 5.2, 5.3, 5.4 and LuaJIT Supported

## Status
- **Type declaration**
    - [ ] builtin-type
        - [x] nil
        - [x] number
        - [ ] integer(lua52 up to)
        - [x] string
        - [x] boolean
        - [ ] function
        - [ ] table
    - [x] compound-type
        - [x] union
        - [x] array
        - [x] tuple
        - [x] dictionary
        - [x] key-value table
    - [ ] class
    - [ ] enum
    - [ ] alias
    - [ ] cast
    - [ ] type coercion(as)
    - [ ] generic function
    - [ ] generic class and method
- **Type check**
    - [x] assign-type-mismatch
    - [ ] return-type-mismatch
    - [ ] param-type-mismatch
    - [ ] field-type-mismatch
    - [ ] table-shape-mismatch
- **Suppert Lua Version**
    - [x] Lua51
    - [ ] Lua52
    - [ ] Lua53
    - [ ] Lua54
    - [ ] LuaJIT
- **performance**
    - Reducing check time
        - [x] Full Analysis of changed file
        - [ ] Incremental Analysis of only the changed parts
- **LSP Support**
    - [x] Diagnostics
    - [ ] Inlay hints
    - [ ] Hover
    - [ ] References
    - [ ] Goto Type Defenition

# Install

## `cargo`
```bash
cargo install typua
```

# Editor Integration

## nvim

### builtin lspconfig
```lua
vim.lsp.enable("typua")
vim.lsp.config("typua", {
    cmd = { "typua", "serve" },
    filetypes = { "lua" },
    root_markers = { ".git", ".typua.toml" },
    settings = {
        typua = {
            workspace = {
                library = {
                    vim.env.VIMRUNTIME,
                }
            }
        },
    },
})
```

# Using with `lua-ls`
`typua` can be used in combination with `lua-ls`

```lua
{
    cmd = { "lua-language-server" },
    filetypes = { "lua" },
        -- omitted...
    settings = {
        Lua = {
            hint = { -- use with typua
                enable = false,
            },
            diagnostics = { -- use with typua
                enable = true,
                disable = {
                    "assign-type-mismatch",
                    "param-type-mismatch",
                    "return-type-mismatch",
                    "undefined-field",
                }
            }
        },
    },
}
```



# Configure
`typua`  detects `.typua.toml` in workingspace root.

on the other hand, use `--config/-c` option like `typua --config your_typua.toml`.

```toml
[workspace]
ignore_dir = ["target"]
use_gitignore = true
```

