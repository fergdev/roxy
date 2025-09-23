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
---@field method string
---@field version string
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
---@field CONNECT string
---@field DELETE string
---@field GET string
---@field HEAD string
---@field OPTIONS string
---@field PATCH string
---@field POST string
---@field PUT string
---@field TRACE string
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

---@class Status
---@field CONTINUE  number
---@field SWITCHING_PROTOCOLS  number
---@field PROCESSING  number
---@field OK  number
---@field CREATED  number
---@field ACCEPTED  number
---@field NON_AUTHORITATIVE_INFORMATION  number
---@field NO_CONTENT  number
---@field RESET_CONTENT  number
---@field PARTIAL_CONTENT  number
---@field MULTI_STATUS  number
---@field ALREADY_REPORTED  number
---@field IM_USED  number
---@field MULTIPLE_CHOICES  number
---@field MOVED_PERMANENTLY  number
---@field FOUND  number
---@field SEE_OTHER  number
---@field NOT_MODIFIED  number
---@field USE_PROXY  number
---@field TEMPORARY_REDIRECT  number
---@field PERMANENT_REDIRECT  number
---@field BAD_REQUEST  number
---@field UNAUTHORIZED  number
---@field PAYMENT_REQUIRED  number
---@field FORBIDDEN  number
---@field NOT_FOUND  number
---@field METHOD_NOT_ALLOWED  number
---@field NOT_ACCEPTABLE  number
---@field PROXY_AUTHENTICATION_REQUIRED  number
---@field REQUEST_TIMEOUT  number
---@field CONFLICT  number
---@field GONE  number
---@field LENGTH_REQUIRED  number
---@field PRECONDITION_FAILED  number
---@field PAYLOAD_TOO_LARGE  number
---@field URI_TOO_LONG  number
---@field UNSUPPORTED_MEDIA_TYPE  number
---@field RANGE_NOT_SATISFIABLE  number
---@field EXPECTATION_FAILED  number
---@field IM_A_TEAPOT  number
---@field MISDIRECTED_REQUEST  number
---@field UNPROCESSABLE_ENTITY  number
---@field LOCKED  number
---@field FAILED_DEPENDENCY  number
---@field TOO_EARLY  number
---@field UPGRADE_REQUIRED  number
---@field PRECONDITION_REQUIRED  number
---@field TOO_MANY_REQUESTS  number
---@field REQUEST_HEADER_FIELDS_TOO_LARGE  number
---@field UNAVAILABLE_FOR_LEGAL_REASONS  number
---@field INTERNAL_SERVER_ERROR  number
---@field NOT_IMPLEMENTED  number
---@field BAD_GATEWAY  number
---@field SERVICE_UNAVAILABLE  number
---@field GATEWAY_TIMEOUT  number
---@field HTTP_VERSION_NOT_SUPPORTED  number
---@field VARIANT_ALSO_NEGOTIATES  number
---@field INSUFFICIENT_STORAGE  number
---@field LOOP_DETECTED  number
---@field NOT_EXTENDED  number
---@field NETWORK_AUTHENTICATION_REQUIRED  number
---@type Status
Status = {
	CONTINUE = 100,
	SWITCHING_PROTOCOLS = 101,
	PROCESSING = 102,
	OK = 200,
	CREATED = 201,
	ACCEPTED = 202,
	NON_AUTHORITATIVE_INFORMATION = 203,
	NO_CONTENT = 204,
	RESET_CONTENT = 205,
	PARTIAL_CONTENT = 206,
	MULTI_STATUS = 207,
	ALREADY_REPORTED = 208,
	IM_USED = 226,
	MULTIPLE_CHOICES = 300,
	MOVED_PERMANENTLY = 301,
	FOUND = 302,
	SEE_OTHER = 303,
	NOT_MODIFIED = 304,
	USE_PROXY = 305,
	TEMPORARY_REDIRECT = 307,
	PERMANENT_REDIRECT = 308,
	BAD_REQUEST = 400,
	UNAUTHORIZED = 401,
	PAYMENT_REQUIRED = 402,
	FORBIDDEN = 403,
	NOT_FOUND = 404,
	METHOD_NOT_ALLOWED = 405,
	NOT_ACCEPTABLE = 406,
	PROXY_AUTHENTICATION_REQUIRED = 407,
	REQUEST_TIMEOUT = 408,
	CONFLICT = 409,
	GONE = 410,
	LENGTH_REQUIRED = 411,
	PRECONDITION_FAILED = 412,
	PAYLOAD_TOO_LARGE = 413,
	URI_TOO_LONG = 414,
	UNSUPPORTED_MEDIA_TYPE = 415,
	RANGE_NOT_SATISFIABLE = 416,
	EXPECTATION_FAILED = 417,
	IM_A_TEAPOT = 418,
	MISDIRECTED_REQUEST = 421,
	UNPROCESSABLE_ENTITY = 422,
	LOCKED = 423,
	FAILED_DEPENDENCY = 424,
	TOO_EARLY = 425,
	UPGRADE_REQUIRED = 426,
	PRECONDITION_REQUIRED = 428,
	TOO_MANY_REQUESTS = 429,
	REQUEST_HEADER_FIELDS_TOO_LARGE = 431,
	UNAVAILABLE_FOR_LEGAL_REASONS = 451,
	INTERNAL_SERVER_ERROR = 500,
	NOT_IMPLEMENTED = 501,
	BAD_GATEWAY = 502,
	SERVICE_UNAVAILABLE = 503,
	GATEWAY_TIMEOUT = 504,
	HTTP_VERSION_NOT_SUPPORTED = 505,
	VARIANT_ALSO_NEGOTIATES = 506,
	INSUFFICIENT_STORAGE = 507,
	LOOP_DETECTED = 508,
	NOT_EXTENDED = 510,
	NETWORK_AUTHENTICATION_REQUIRED = 511,
}

---@class Response
---@field status integer
---@field version string
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
