pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local ext = {
	request = function(flow)
		flow.request.body.clear()
	end,

	response = function(flow)
		flow.response.body.clear()
	end,
}
Extensions = { ext }
