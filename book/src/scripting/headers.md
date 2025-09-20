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

### Set header

{{#tabs global="language"}}
{{#tab name=JS}}

```js
flow.request.headers.set("X-Header1", "request");
```

{{#endtab}}
{{#tab name=Lua}}

```lua
flow.request.headers:set("X-Header1", "request")
```

{{#endtab}}
{{#tab name=Python}}

```py
flow.request.headers.set("X-Header1", "request")
```

{{#endtab}}
{{#endtabs}}

### Append Header

{{#tabs global="language"}}
{{#tab name=JS}}

```js
flow.request.headers.append("X-Header1", "request");
```

{{#endtab}}
{{#tab name=Lua}}

```lua
flow.request.headers:append("X-Header1", "request")
```

{{#endtab}}
{{#tab name=Python}}

```py
flow.request.headers.append("X-Header1", "request")
```

{{#endtab}}
{{#endtabs}}

### Remove a header

{{#tabs global="language"}}
{{#tab name=JS}}

```js
flow.request.headers.delete("X-Header1");
flow.request.headers.set("X-header2", undefined);
flow.request.headers.set("X-header3", null);
```

{{#endtab}}
{{#tab name=Lua}}

```lua
flow.request.headers:delete("X-Header");
flow.request.headers["X-Header2"] = nil
```

{{#endtab}}
{{#tab name=Python}}

```py
flow.request.headers.delete("X-Header1")
flow.request.headers["X-Header2"] = None
del flow.request.headers["X-header3"]
```

{{#endtab}}
{{#endtabs}}
