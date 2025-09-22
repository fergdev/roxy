pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local version_set = {
	function(flow)
		flow.request.version = "HTTP/3.0"
	end,
	function(flow)
		flow.response.version = "HTTP/3.0"
	end,
}
Extensions = { version_set }
