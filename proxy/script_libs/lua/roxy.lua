---@meta
-- Roxy scripting API types for IDEs (EmmyLua / Lua LS).
-- Place this file in your repo and add its folder to:
-- VS Code:  Settings → Lua › Workspace: Library
-- JetBrains: Settings → Languages & Frameworks → Lua → Libraries

---@class Extension
---@field start fun()?               # Optional start handler
---@field request fun(flow: Flow)?   # Optional request handler
---@field response fun(flow: Flow)?  # Optional response handler
---@field stop fun()?                # Optional stop handler

---@class Flow
---@field request Request
---@field response Response?

---@class Request
---@field url URL
---@field method Method
---@field version Version
---@field headers Headers
---@field body Body
---@field trailers Headers?

---@class URL
---@field protocol string
---@field authority string?
---@field username string?
---@field password string?
---@field host string?
---@field hostname string?
---@field port number?
---@field path string?
---@field search string
---@field search_params URLSearchParams
---
---@class URLSearchParams
---@field set fun(self: URLSearchParams, key: string, value: string)
---@field append fun(self: URLSearchParams, key: string, value: string)
---@field clear fun(self: URLSearchParams)
---@field delete fun(self: URLSearchParams, key: string)

---@class Protocol
---@field HTTPS string
---@field HTTP string
---@type Protocol
Protocol = {
	HTTP = "http",
	HTTPS = "https",
}

---@class Version
---@field HTTP09 string
---@field HTTP10 string
---@field HTTP11 string
---@field HTTP2 string
---@field HTTP3 string
---@type Version
Version = {
	HTTP09 = "HTTP/0.9",
	HTTP10 = "HTTP/1.0",
	HTTP11 = "HTTP/1.1",
	HTTP2 = "HTTP/2",
	HTTP3 = "HTTP/3",
}

---@class Method
---@field GET string
---@field POST string
---@field PUT string
---@field DELETE string
---@type Method
Method = {
	CONNECT = "CONNECT",
	DELETE = "DELETE",
	GET = "GET",
	HEAD = "HEAD",
	OPTIONS = "OPTIONS",
	PATCH = "PATCH",
	POST = "POST",
	PUT = "PUT",
	TRACE = "TRACE",
}

---@class Response
---@field status integer
---@field version Version
---@field headers Headers
---@field body Body
---@field trailers Headers?

---@class Body
---@field text string         # UTF-8 text view of the body (empty string if binary)
---@field raw string          # Raw bytes view (Lua string)
---@field clear fun()         # Clears body to empty
---@field is_empty boolean    # True if body length is zero

---@class Headers
---@field get fun(self: Headers, key: string): string|nil
---@field set fun(self: Headers, key: string, value: string)
---@field append fun(self: Headers, key: string, value: string)
---@field delete fun(self: Headers, key: string)
---@field has fun(self: Headers, key: string): boolean
---@field clear fun()

---@type Extension[]  # Global array discovered by Roxy
Extensions = {}

---@class Roxy
---@field notify fun(severity: integer, message: string)

---@type Roxy
Roxy = Roxy
