---@type number | string
local x = "hello"

if type(x) == "string" then
elseif type(x) == "number" then
end

---@type number?
local y = nil

if y ~= nil then
    local yy = y
end

---@type number|string|boolean
local y = true
if type(y) == "string" then
    local yy = y   -- expected string
elseif type(y) ~= "number" then
    local yy = y  -- expected boolean
else
    local yy = y   -- expected number
end

local yy = y
local as_path
