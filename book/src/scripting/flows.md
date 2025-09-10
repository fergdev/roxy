# Flows

A **Flow** is the object your scripts receive for each HTTP exchange. It exposes:

- `flow.request`: the incoming request (method, URL, headers, body, trailers, version)  
- `flow.response`: the outgoing response (status, headers, body, trailers, version)  

You can read/modify either side during interception.

---

## Interception lifecycle

- **Request phase:** runs before the upstream request is sent.  
  You can rewrite method, URL, headers, or body — or even synthesize a response and short-circuit the request.

- **Response phase:** runs after a response is available.  
  You can edit status, headers, body, or trailers before the client sees it.

---

## Examples

{{#tabs global="language"}}
{{#tab name="Lua"}}

```lua
-- Request interception
function request(flow)
  local h = flow.request.headers
  print(h["host"])
  h["X-Trace"] = "abc123"

  -- Rewrite body
  flow.request.body.text = flow.request.body.text .. " appended from Lua"
end

-- Response interception
function response(flow)
  local h = flow.response.headers
  h["Server"] = "LuaProxy"
  flow.response.body.text = "overwritten response"
end
```
{{#endtab}}
{{#tab name=“JS”}}// Request interception
```js
function request(flow) {
  const h = flow.request.headers;
  console.log(h.get("host"));
  h.set("X-Trace", "abc123");

  flow.request.body.text = flow.request.body.text + " appended from JS";
}

function response(flow) {
  const h = flow.response.headers;
  h.set("Server", "JsProxy");
  flow.response.body.text = "overwritten response";
}
```

{{#endtab}}
{{#tab name=“Python”}}
```py
def request(flow):
    h = flow.request.headers
    print(h["host"])
    h["X-Trace"] = "abc123"

    # Rewrite body
    flow.request.body = flow.request.body + " appended from Python"

# Response interception
def response(flow):
    h = flow.response.headers
    h["Server"] = "PyProxy"
    flow.response.body = "overwritten response"

```
{{#endtab}}
{{#endtabs}}

