pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local query_set = {
	request = function(flow)
		if flow.request.url.search_params["foo"] == "bar" then
			flow.request.url.search_params:set("foo", "baz")
		end
	end,
}
Extensions = { query_set }
