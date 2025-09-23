pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local query_clear = {
	request = function(flow)
		if flow.request.url.search_params["foo"] == "bar" then
			flow.request.url.search_params:clear()
		end
	end,
}
Extensions = { query_clear }
