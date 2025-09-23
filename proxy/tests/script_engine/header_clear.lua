pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local header_clear = {
	request = function(flow)
		flow.request.headers:clear()
	end,
	response = function(flow)
		flow.response.headers:clear()
	end,
}
Extensions = { header_clear }
