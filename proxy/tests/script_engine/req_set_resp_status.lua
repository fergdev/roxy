pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local req_set_resp_status = {
	function(flow)
		flow.response.status = Status.NOT_FOUND
	end,
}
Extensions = { req_set_resp_status }
