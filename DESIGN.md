# Type Annotations
## Basic Types
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

## Collection Types

| Type            | Annotation                              |
| ------          | -----------                             |
| Union           | `Type1 \| Type2`                        |
| Array           | `ValueType[]`                           |
| Tuple           | `[ValueType1, ValueType2, ...]`         |
| Dictionary      | `{ [string]: ValueType }`               |
| Key-Value Table | `table<KeyType, ValueType>`             |
| Table Literal   | `{ key1: ValueType, key2: ValueType2 }` |
| Function        | `fun(Param: Type): ReturnType`          |

## Class
### Syntax
```lua
---@class [(exact)] <name>[: <parent>[, <parent>...]]
```

### Define a Class
```lua
---@class Car
local Car = {}
```

### Inheritance
```lua
---@class Vehicle
local Vehcle = {}

---@class Plane: Vehicle
local Plane = {}
```


# LSP Capabilities
## Diagnostics
## Inlay Hints

### Assign Literal
No Display
```lua
local a = 1
local b = "hello"
```

### Assign Expression
```lua
local x = 12
local y = "hello"
local a: number = x
local b: string = y
```

### Assign Anonimous Function
```lua
---@param x number
---@param y number
---@return number
local f = function(x: number, y: number)
	return x + y
end
local result: number = f(x: 12, y: 34)
```

### Array Index
```lua
local ary = {
	[1] "A",
	[2] "B",
	[3] "C",
}
local ary = {
	 "A", [1]
	 "B", [2]
	 "C", [3]
}
```
## Hover
## References
