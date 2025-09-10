# Headers

HTTP headers are **case-insensitive**, **order-preserving**, and allow **multiple fields with the same name**.

## Quick API

- `h[name]` / `h.get(name)` → folded string (values joined with `", "`).
- `h[name] = value` / `h.set(name, value)` → replace all fields for that name.
- `del h[name]` / `h.delete(name)` → remove all fields for that name.
- `h.get_all(name)` → list of values in order.
- `h.set_all(name, values)` → explicit multi-field set.
- `h.insert(index, name, value)` → insert raw field at index.
- `h.items(multi=false)` → iterate (raw if `multi=true`).

## Common tasks

### Read/Write a header

{{#tabs global="language"}}
{{#tab name="Lua"}}

```lua
local h = flow.request.headers
print(h["host"])
h["X-Trace"] = "abc123"
```

{{#endtab}}
{{#tab name=“JS”}}

```js
const h = flow.request.headers;
console.log(h.get("host"));
h.set("X-Trace", "abc123");
```

{{#endtab}}

{{#tab name=“Python”}}

```py
h = flow.request.headers
print(h["host"])
h["X-Trace"] = "abc123"
```

{{#endtab}}
{{#endtabs}}
