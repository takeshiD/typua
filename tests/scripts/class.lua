---@class (exact) Container
---@field x number
---@field y string

---@type Container

local c = {}

---@type Container
local Container = {}

---@class (exact) Point2D
---@field x number
---@field y number
local p2d = {}
p2d.x = 1
p2d.y = 2
p2d.z = 12

---@class (exact) Point3D
---@field x number
---@field y number
---@field z number
local p3d = {}
p3d.x = 1
p3d.y = 2
p3d.z = 2

---@class (exact) Stack
---@field private _stack number[]
---@field new fun(): Stack
---@field pop fun(self): number
---@field push fun(self, val: number)
local Stack = {}
Stack.__index = Stack

function Stack.new()
    local obj = {_stack = {}}
    return setmetatable(obj, {__index = Stack})
end

function Stack:pop()
    return table.remove(self._stack)
end

function Stack:push(val)
    return table.insert(self._stack, val)
end
