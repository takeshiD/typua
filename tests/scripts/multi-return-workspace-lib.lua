---@class Wrapper
---@field value number

---@return Wrapper result
---@return string? err
local function make_wrapper(x)
    if x then
        return { value = 1 }, nil
    end

    return { value = 0 }, "missing"
end

return make_wrapper
