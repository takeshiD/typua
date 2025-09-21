---@generic T
---@param x T
---@return fun(): T
local function const(x)
    return function()
        return x
    end
end


---@generic T
---@class Container<T>
local Container = {}
Container.__index = Container

---@generic T
---@type fun(self: Container<T>): T
Container.get_value = const("if this shown")

---@generic T
---@param value T
---@return Container<T>
function Container.new(value)
    local self = setmetatable({}, Container)
    return self
end

---@generic T
---@type fun(self: Container<T>): T
function Container:get()
    return self:get_value()
end


---@generic T
---@type fun(self: Container<unknown>, new_value: T): Container<T>
function Container:set_value(new_value)
    return Container.new(new_value)
end

local x = Container.new(10):get()
local y = Container:set_value("hello")
local z = Container:set_value({})
print(x)
print(y)
