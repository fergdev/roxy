pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local body_set = {
	request = function(flow)
		flow.request.body.text = "rewrite request"
	end,
	response = function(flow)
		flow.response.body.text = "rewrite response"
	end,
}
Extensions = { body_set }
