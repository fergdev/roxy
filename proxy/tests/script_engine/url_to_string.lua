pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local url_to_string = {
	request = function(flow)
		flow.request.body.text = tostring(flow.request.url)
	end,
}
Extensions = { url_to_string }
