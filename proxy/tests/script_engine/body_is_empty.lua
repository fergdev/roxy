pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local body_is_empty = {
	request = function(flow)
		if flow.request.body.is_empty then
			flow.request.body.text = "empty request"
		end
	end,
	response = function(flow)
		if flow.response.body.is_empty then
			flow.response.body.text = "empty response"
		end
	end,
}
Extensions = { body_is_empty }
