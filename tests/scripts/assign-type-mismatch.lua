local m = {}
---@type integer[]
m.ints = {}


---@class A
---@field x A

---@type A
local t

t.x = {}

---@class A
---@field x integer

---@type A
local t


---@class A
---@field x integer

---@type A
local t

---@type boolean
local y


---@class A
local m

m.x = 1

---@type A
local t


---@class A
local m

---@type integer
m.x = 1

---@class A
local mt

---@type integer
mt.x = 1

function mt:init()
end

---@class A
---@field x integer

---@type A
local t = {
}

---@type boolean[]
local t = {}

t[5] = nil

---@type table<string, true>
local t = {}

t['x'] = nil

---@type [boolean]
local t = { [1] = nil }

t = nil

local t = { true }

t[1] = nil

---@class A
local t = {
    x = 1
}

---@type number
local t

t = 1

---@type number
local t

---@type integer
local y

t = y

---@class A
local m

---@type number
m.x = 1


local n

if G then
    n = {}
else
    n = nil
end

local t = {
    x = n,
}

---@type boolean[]
local t = {}

---@type boolean?
local x

t[#t+1] = x

---@type number
local n
---@type integer
local i

---@type number
local n
---@type integer
local i

i = n

---@type number|boolean
local nb

---@type number
local n


---@type number|boolean
local nb

---@type number
local n

n = nb

---@class Option: string

---@param x Option
local function f(x) end

---@type Option
local x = 'aaa'

f(x)

---@type number
local x = 'aaa'
---@class X

---@class A
local mt = G

---@type X
mt._x = nil

---@type number?
local nb

---@type number
local n

n = nb

---@type number|nil
local nb

---@type number
local n

n = nb
config.set(nil, 'Lua.type.weakNilCheck', false)

---@class A
local a = {}

---@class B

---@class A
local a = {}

---@class B: A
local b = a

---@class A
local a = {}
a.__index = a

---@class B: A
local b = setmetatable({}, a)

---@class A
local a = {}

---@class B: A
local b = setmetatable({}, {__index = a})

---@class A
local a = {}

---@class B
local b = setmetatable({}, {__index = a})

---@class A
---@field x number?
local a

---@class B
---@field x number
local b

b.x = a.x

---@class A
---@field x number?
local a

---@type number
local t

t = a.x

local mt = {}
mt.x = 1
mt.x = nil

---@type number
local x = G

---@generic T
---@param x T
---@return T
local function f(x)
    return x
end

---@alias test boolean

---@type test

---@class MyClass
local MyClass = {}

function MyClass:new()
    ---@class MyClass
    local myObject = setmetatable({
        initialField = true
    }, self)

    print(myObject.initialField)
end

---@class T
local t = {
    x = nil
}

t.x = 1

---@type {[1]: string, [10]: number, xx: boolean}
local t = {
    true,
    [10] = 's',
    xx = 1,
}

---@type boolean[]
local t = { 1, 2, 3 }

---@type boolean[]
local t = { true, false, nil }

---@type boolean|nil
local x

---@type boolean[]
local t = { true, false, x }

---@enum Enum
local t = {
    x = 1,
    y = 2,
}

---@type Enum
local y

---@type integer
local x = y

---@type string|string[]|string[][]
local t = {{'a'}}

local A = "Hello"
local B = "World"

---@alias myLiteralAliases `A` | `B`

---@type myLiteralAliases
local x = A

local enum = { a = 1, b = 2 }

---@type { [integer] : boolean }
local t = {
    [enum.a] = 1,
    [enum.b] = 2,
    [3] = 3,
}

---@class SomeClass
---@field [1] string
-- ...

---@param some_param SomeClass|SomeClass[]
local function some_fn(some_param) return end

some_fn { { "test" } } -- <- diagnostic: "Cannot assign `table` to `string`."

---@type string[]
local arr = {
    3,
}

---@type (string|boolean)[]
local arr2 = {
    3, -- no warnings
}

local t = {}
t.a = 1
t.a = 2
return t
