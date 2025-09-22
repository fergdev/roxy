pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local url_path = {
	request = function(flow)
		if flow.request.url.path == "/some/path" then
			flow.request.url.path = "/another/path"
		end
	end,
}
Extensions = { url_path }
