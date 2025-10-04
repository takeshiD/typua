---@return number result
---@return string err
local function multi(x)
    if x > 0 then
        return x, 1
    else
        return 0, "ok"
    end
end

return multi(-1)
