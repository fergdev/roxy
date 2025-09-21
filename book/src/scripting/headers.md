# Headers

HTTP headers are **case-insensitive**, **order-preserving**, and allow **multiple fields with the same name**.

## API

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

### Test if value is present

{{#tabs global="language"}}
{{#tab name=JS}}

```js
if (flow.request.headers.has("X-Header")) {
  console.log("header is present");
}
```

{{#endtab}}
{{#tab name=Lua}}

```lua
if (flow.request.headers:has("X-Header")) then
  print("header is present")
end
```

{{#endtab}}
{{#tab name=Python}}

```py
if flow.request.headers.has("X-Header"):
  print("header is present")
```

{{#endtab}}
{{#endtabs}}
