pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local request_set_method = {
	request = function(flow)
		if flow.request.method == Method.GET then
			flow.request.method = Method.POST
		end
	end,
}
Extensions = { request_set_method }
