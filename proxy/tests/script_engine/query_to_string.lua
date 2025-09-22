pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local query_to_string = {
	request = function(flow)
		flow.request.body.text = tostring(flow.request.url.search_params)
	end,
}
Extensions = { query_to_string }
