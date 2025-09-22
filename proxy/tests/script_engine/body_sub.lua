pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local body_sub = {
	request = function(flow)
		flow.request.body.text = string.gsub(flow.request.body.text, "replaceme", "gone")
	end,

	response = function(flow)
		flow.response.body.text = string.gsub(flow.response.body.text, "to_go", "it_went")
	end,
}
Extensions = { body_sub }
