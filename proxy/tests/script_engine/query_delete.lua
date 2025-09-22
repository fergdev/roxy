pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local query_delete = {
	request = function(flow)
		if flow.request.url.search_params["foo"] == "bar" then
			flow.request.url.search_params:delete("foo")
		end
	end,
}
Extensions = { query_delete }
