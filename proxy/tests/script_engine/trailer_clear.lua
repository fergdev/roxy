pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local trailer_clear = {
	request = function(flow)
		flow.request.trailers:clear()
	end,
	response = function(flow)
		flow.response.trailers:clear()
	end,
}
Extensions = { trailer_clear }
