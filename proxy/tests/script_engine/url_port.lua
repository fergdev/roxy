pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local url_port = {
	request = function(flow)
		if flow.request.url.port == 1234 then
			flow.request.url.port = 8080
		end
	end,
}
Extensions = { url_port }
