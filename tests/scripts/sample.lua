---@class Person
---@field x number
---@field y string

---@type Person
local P = {}
P.x = 12
P.y = 12

-- type mismatch
---@type number
local isNum = false

---@class myClass
local myClass = {}

-- undefined field "hello"
myClass.hello()


-- need check nil
---@class Bicycle
---@field move function

---@type Bicycle|nil
local bicycle

-- need to make sure bicycle isn't nil first
bicycle.move()
