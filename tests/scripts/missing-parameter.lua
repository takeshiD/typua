local function x(a, b)
    return a, b
end
x(1)

---@param a integer
---@param b integer
local function x(a, b)
    return a, b
end
x(1)

---@param a integer
---@param b integer
local function x(a, b)
    return a, b
end
x()

---@param a integer
---@param b integer
---@param ... integer
local function x(a, b, ...)
    return a, b, ...
end
x(1, 2)

---@param a integer
---@param b integer
local function f(a, b)
end

f(...)

---@param a integer
---@param b integer
local function f(a, b)
end

local function return2Numbers()
    return 1, 2
end

f(return2Numbers())

---@param a integer
---@param b? integer
local function x(a, b)
    return a, b
end
x(1)

---@param b integer?
local function x(a, b)
    return a, b
end
x(1)

---@param b integer|nil
local function x(a, b)
    return a, b
end
x(1)

local t = {}

function t:init() end

t.init()
