# Headers (Spec)

A case-insensitive, order-preserving header collection that permits multiple fields of the same name and renders to an HTTP/1 header block.

## 1) Requirements (MUST)

- Keys are **case-insensitive** for lookup/mutation (ASCII lowercase comparison).
- **Raw order preserved** (insertion order of fields).
- **Multiple fields** with the same name are allowed.
- Byte rendering uses **HTTP/1 header line format**: `Name: value\r\n` per field, **no trailing blank line**.

## 2) API (language-agnostic)

- `h[name] -> str` — folded value (join with `", "`).
- `h[name] = str|bytes` — replace **all** fields for `name` with a single field appended at end.
- `del h[name]` — remove **all** fields for `name`.
- `get_all(name) -> list[str]` — raw values in order (no folding).
- `set_all(name, values: Iterable[str|bytes])` — explicit multi-fields (append in order).
- `insert(index, name, value)` — insert raw field at `index` (0-based).
- `items(multi=false)` — iterator:
  - `false`: logical (folded) items in order of first appearance by lowercase name.
  - `true`: raw `(name, value)` pairs in stored order.
- `bytes(h)` / `toBytes(h)` — CRLF per line, no final blank line.

## 3) Language contracts

{{#tabs global="language"}}

{{#tab name="Lua"}}

| Topic     | API |
|-----------|-----|
| Type      | `Headers` userdata |
| Construct | `Headers.new({{"Name","value"}, ...})` |
| Access    | `h["Name"]`, `h["Name"] = v`, `h["Name"] = nil` |
| Methods   | `h:get_all(name)`, `h:set_all(name, vals)`, `h:insert(i,n,v)`, `h:items(multi)`, `h:to_bytes()` |
{{#endtab}}

{{#tab name="JS"}}

| Topic     | API |
|-----------|-----|
| Type      | `class Headers` |
| Construct | `new Headers([["Name","value"], ...])` |
| Access    | `h.get(name)`, `h.set(name, value)`, `h.delete(name)` |
| Methods   | `getAll`, `setAll`, `insert`, `items(multi=false)`, `toBytes` |
{{#endtab}}

{{#tab name="Python"}}

| Topic     | API |
|-----------|-----|
| Type      | `class Headers` |
| Construct | `Headers([(b"Name", b"value"), ...])` |
| Access    | `h["Name"]`, `h["Name"] = v`, `del h["Name"]` |
| Methods   | `get_all`, `set_all`, `insert`, `items(multi=False)`, `bytes(h)` |
{{#endtab}}

{{#endtabs}}

## 4) Conformance tests (H-01…H-08)

Implement these **black-box** tests in each language. When all pass, tick the checklist.

**H-01 Case-insensitive lookup & fold**  
Given: `[("Host","example.com"),("accept","text/html"),("ACCEPT","application/xml")]`  
Expect:  

- `get("host") == "example.com"`  
- `get("Accept") == "text/html, application/xml"`  
- `get_all("accept") == ["text/html","application/xml"]`

**H-02 Replace via assignment / set**  
From H-01, set `"Accept" = "application/json"` → `get_all("accept") == ["application/json"]`; raw ends with one `Accept`.

**H-03 set_all order**  
`set_all("Set-Cookie", ["a=1","b=2"])` → `get_all("set-cookie") == ["a=1","b=2"]`; folded `"a=1, b=2"`.

**H-04 delete**  
Delete `"Accept"` → `get_all("accept") == []`, others intact.

**H-05 insert at index**  
`insert(1, "X-Debug", "on")` → raw index 1 equals `("X-Debug","on")`.

**H-06 bytes rendering**  
Rendered bytes are joined `Name: value\r\n` per raw field, no extra blank line.

**H-07 items(multi=false)**  
Yields folded logical items in **first-appearance** order by lowercase name.

**H-08 mixed str/bytes**  
Accept both `str`/`bytes` for input; read returns `string` (Lua/JS) or `str` (Py).

## 5) Checklist

- [ ] Lua
- [ ] JS
- [ ] Python
