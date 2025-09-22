pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local header_to_string = {
	request = function(flow)
		flow.request.body.text = tostring(flow.request.headers)
	end,
	response = function(flow)
		flow.response.body.text = tostring(flow.response.headers)
	end,
}
Extensions = { header_to_string }
