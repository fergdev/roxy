pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local req_set_resp_body = {
	function(flow)
		flow.response.body.text = "early return"
	end,
}
Extensions = { req_set_resp_body }
