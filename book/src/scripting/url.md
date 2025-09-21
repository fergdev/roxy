# URL

URL is a parameter on thew Flow.response object that allows you to get or set the URL of the request.

## API

### Host

{{#tabs global="language"}}
{{#tab name=JS}}

```js
if (flow.request.url.host == "localhost:1234") {
  flow.request.url.host = "example.com:4321"
}
```

{{#endtab}}
{{#tab name=Lua}}

```lua
if (flow.request.url.host == "localhost:1234") then
  flow.request.url.host = "example.com:4321"
end
```

{{#endtab}}
{{#tab name=Python}}

```py
if flow.request.url.host == "localhost:1234":
    flow.request.url.host = "example.com:4321"
```

{{#endtab}}
{{#endtabs}}

### Hostname

{{#tabs global="language"}}
{{#tab name=JS}}

```js
if (flow.request.url.host == "localhost") {
  flow.request.url.host = "example.com"
}
```

{{#endtab}}
{{#tab name=Lua}}

```lua
if (flow.request.url.host == "localhost") then
  flow.request.url.host = "example.com"
end
```

{{#endtab}}
{{#tab name=Python}}

```py
if flow.request.url.host == "localhost":
    flow.request.url.host = "example.com"
```

{{#endtab}}
{{#endtabs}}

### Port

{{#tabs global="language"}}
{{#tab name=JS}}

```js
if (flow.request.url.port == 80) {
  flow.request.url.port = 8080
}
```

{{#endtab}}
{{#tab name=Lua}}

```lua
if flow.request.url.port == 80 then
  flow.request.url.port = 8080
end
```

{{#endtab}}
{{#tab name=Python}}

```py
if flow.request.url.port == 80:
    flow.request.url.port = 8080
```

{{#endtab}}
{{#endtabs}}

### Username

{{#tabs global="language"}}
{{#tab name=JS}}

```js
if (flow.request.url.username == "dave") {
  flow.request.url.username = "damo";
}
```

{{#endtab}}
{{#tab name=Lua}}

```lua
if flow.request.url.username == "dave" then
  flow.request.url.username = "damo"
end
```

{{#endtab}}
{{#tab name=Python}}

```py
if flow.request.url.username == "dave":
    flow.request.url.username = "damo"
```

{{#endtab}}
{{#endtabs}}

### Password

{{#tabs global="language"}}
{{#tab name=JS}}

```js
if (flow.request.url.password == "1234") {
  flow.request.url.password = "abcd";
}
```

{{#endtab}}
{{#tab name=Lua}}

```lua
if flow.request.url.password == "1234" then
  flow.request.url.password = "abcd"
end
```

{{#endtab}}
{{#tab name=Python}}

```py
if flow.request.url.password == "1234":
    flow.request.url.password = "abcd"
```

{{#endtab}}
{{#endtabs}}

### Authority

{{#tabs global="language"}}
{{#tab name=JS}}

```js
if (flow.request.url.authority == "dave:1234@localhost:1234") {
  flow.request.url.authority = "damo:abcd@localhost:4321";
}
```

{{#endtab}}
{{#tab name=Lua}}

```lua
if flow.request.url.authority == "dave:1234@localhost:1234" then
  flow.request.url.authority = "damo:abcd@localhost:4321"
end
```

{{#endtab}}
{{#tab name=Python}}

```py
if flow.request.url.authority == "dave:1234@localhost:1234":
    flow.request.url.authority = "damo:abcd@localhost:4321"
```

{{#endtab}}
{{#endtabs}}

### Path

{{#tabs global="language"}}
{{#tab name=JS}}

```js
if (flow.request.url.path == "/some/path") {
  flow.request.url.path = "/another/path";
}
```

{{#endtab}}
{{#tab name=Lua}}

```lua
if flow.request.url.path == "/some/path" then
  flow.request.url.path = "/another/path"
end
```

{{#endtab}}
{{#tab name=Python}}

```py
if flow.request.url.path == "/some/path":
    flow.request.url.path = "/another/path"
```

{{#endtab}}
{{#endtabs}}

### Scheme

{{#tabs global="language"}}
{{#tab name=JS}}

```js
if (flow.request.url.protocol == "http") {
  flow.request.url.protocol = "https";
}
```

{{#endtab}}
{{#tab name=Lua}}

```lua
if flow.request.url.scheme == "http" then
  flow.request.url.scheme = "https"
end
```

{{#endtab}}
{{#tab name=Python}}

```py
if flow.request.url.scheme == "http":
    flow.request.url.scheme = "https"
```

{{#endtab}}
{{#endtabs}}
