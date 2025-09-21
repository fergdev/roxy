# Body

HTTP bodies can be **binary or text**, may be **empty**, and can be **read or replaced** by scripts.

## API

### Set text

{{#tabs global="language"}}
{{#tab name=JS}}

```js
flow.request.body.text = "new request body";
flow.response.body.text = "new response body";
```

{{#endtab}}
{{#tab name=Lua}}

```lua
flow.request.body.text = "new request body"
flow.response.body.text = "new response body"
```

{{#endtab}}
{{#tab name=Python}}

```py
flow.request.body.text = "new request body"
flow.response.body.text = "new response body"
```

{{#endtab}}
{{#endtabs}}

### Get text

{{#tabs global="language"}}
{{#tab name=JS}}

```js
let req_text = flow.request.body.text;
let res_body = flow.response.body.text;
```

{{#endtab}}
{{#tab name=Lua}}

```lua
local req_text = flow.request.body.text;
local res_body = flow.response.body.text;
```

{{#endtab}}
{{#tab name=Python}}

```py
req_text = flow.request.body.text;
res_body = flow.response.body.text;
```

{{#endtab}}
{{#endtabs}}

### Set bytes

{{#tabs global="language"}}
{{#tab name=JS}}

```js
flow.request.body.bytes = new Uint8Array([0xde, 0xad, 0xbe, 0xef]);
```

{{#endtab}}
{{#tab name=Lua}}

```lua
flow.request.body.bytes = string.char(0xde, 0xad, 0xbe, 0xef)
```

{{#endtab}}
{{#tab name=Python}}

```py
flow.request.body.bytes = b"\xde\xad\xbe\xef"
```

{{#endtab}}
{{#endtabs}}

### Get bytes

{{#tabs global="language"}}
{{#tab name=JS}}

```js
const bytes = flow.request.body.bytes
```

{{#endtab}}
{{#tab name=Lua}}

```lua
local bytes = flow.request.body.bytes
```

{{#endtab}}
{{#tab name=Python}}

```py
bytes = flow.request.body.bytes
```

{{#endtab}}
{{#endtabs}}

### Clear body

{{#tabs global="language"}}
{{#tab name=JS}}

```js
flow.request.body.bytes = new Uint8Array([0xde, 0xad, 0xbe, 0xef]);
```

{{#endtab}}
{{#tab name=Lua}}

```lua
flow.request.body.bytes = string.char(0xde, 0xad, 0xbe, 0xef)
```

{{#endtab}}
{{#tab name=Python}}

```py
flow.request.body.bytes = b"\xde\xad\xbe\xef"
```

{{#endtab}}
{{#endtabs}}

### Length

{{#tabs global="language"}}
{{#tab name=JS}}

```js
const len = flow.request.body.len
```

{{#endtab}}
{{#tab name=Lua}}

```lua
local len = flow.request.body.len
```

{{#endtab}}
{{#tab name=Python}}

```py
len = flow.request.body.len
```

{{#endtab}}
{{#endtabs}}

### Is empty

{{#tabs global="language"}}
{{#tab name=JS}}

```js
const isEmpty = flow.request.body.isEmpty
```

{{#endtab}}
{{#tab name=Lua}}

```lua
local isEmpty = flow.request.body.isEmpty
```

{{#endtab}}
{{#tab name=Python}}

```py
isEmpty = flow.request.body.isEmpty
```

{{#endtab}}
{{#endtabs}}
