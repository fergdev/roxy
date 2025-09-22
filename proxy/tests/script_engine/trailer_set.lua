pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local trailer_set = {
	request = function(flow)
		flow.request.trailers:set("X-Trailer1", "request")
	end,
	response = function(flow)
		flow.response.trailers:set("X-Trailer1", "response")
	end,
}
Extensions = { trailer_set }
