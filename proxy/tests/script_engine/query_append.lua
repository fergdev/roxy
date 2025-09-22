pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local query_append = {
	request = function(flow)
		if flow.request.url.search_params["foo"] == "bar" then
			flow.request.url.search_params:append("foo", "baz")
		end
	end,
}
Extensions = { query_append }
