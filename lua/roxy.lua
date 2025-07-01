-- roxy.lua
-- This module provides type definitions and helper examples for Roxy proxy scripts.

---@class Headers
---@field [string] string

---@class InterceptedRequest
---@field url string
---@field headers Headers
---@field body string

---@class InterceptedResponse
---@field headers Headers
---@field body string

local roxy = {}

-- TODO: this is out of date, the whole flow is passed here
--- Example: Intercept a request
---@param req InterceptedRequest
---@return InterceptedRequest
function roxy.intercept_request(req)
	print("Intercepting request to: " .. req.url)
	-- Example: Add a custom header
	req.headers["X-Roxy-Injected"] = "true"
	return req
end

--- Example: Intercept a response
---@param res InterceptedResponse
---@return InterceptedResponse
function roxy.intercept_response(res)
	print("Intercepting response with body size: " .. #res.body)
	return res
end

return roxy
