pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local req_set_body_based_on_req = {
	response = function(flow)
		if flow.request.url.host == "example.com" then
			flow.response.body.text = "intercepted"
		end
	end,
}
Extensions = { req_set_body_based_on_req }
