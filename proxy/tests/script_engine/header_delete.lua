pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local header_append = {
	request = function(flow)
		flow.request.headers:delete("X-Header1")
		flow.request.headers["X-Header2"] = nil
		flow.request.headers["X-Header3"] = nil -- TODO: there is no 3rd way to delete headers in lua
	end,
	response = function(flow)
		flow.response.headers:delete("X-Header1")
		flow.response.headers["X-Header2"] = nil
		flow.response.headers["X-Header3"] = nil -- TODO: there is no 3rd way to delete headers in lua
	end,
}
Extensions = { header_append }
