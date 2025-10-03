---@param x number
---@param y number?
---@return number
local function f(x, y)
    if y == nil then
        return -100
    end
    local z = y
	return x + y
end

local ret1 = f(12, 12)
local ret2 = f(12, nil)
