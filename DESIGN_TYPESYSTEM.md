# Gradual Typing

## ?(Unknown) Type
If programmer omits a expression's type annotation, gradual type system will judge it is ?(Unknown) type.

- Example: All Unknown
```lua
function func(x, y)
    return x + y
end
```

In this case, when function `func` has 2 params, 1 return value.
Type System judge each params and return value is `Unknown` type.


- Example: Partial Unknown
```lua
---param x number
function func(x, y)
    return x + y
end
```
In above case, Type System judge `x` is `number` type but `y` and retuyn value is `Unknown`

## Type Consistency
### Restriction Operator $\sigma | \tau$


# TypeCheck

# TypeInference
