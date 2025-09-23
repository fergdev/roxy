pcall(require, "../../script_libs/lua/roxy.lua")
local count = 0
---@type Extension
local start_invoked = {
	start = function()
		count = 10
	end,
	request = function(flow)
		flow.request.body.text = tostring(count)
		count = count + 1
	end,
	response = function(flow)
		flow.response.body.text = tostring(count)
	end,
}
Extensions = { start_invoked }
