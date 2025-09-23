pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local response_set_status = {
	response = function(flow)
		flow.response.status = Status.NOT_FOUND
	end,
}
Extensions = { response_set_status }
