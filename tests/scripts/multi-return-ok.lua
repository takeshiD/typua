---@param x number
---@return number? result
---@return string? err
local function multi(x)
    if x > 0 then
        return x, nil
    else
        return nil, "error"
    end
end

local value, err = multi(1)
return value, err
