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
