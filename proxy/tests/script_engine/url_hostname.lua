pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local url_hostname = {
	request = function(flow)
		if flow.request.url.hostname == "localhost" then
			flow.request.url.hostname = "example.com"
		end
	end,
}
Extensions = { url_hostname }
