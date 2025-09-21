# Notify

Notifications can be sent to the UI through the notify API.

Notifications have 5 levels: `debug`, `info`, `warning`, `error` and `trace`.
Filters can be set in the settings for filtering notifications.

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
