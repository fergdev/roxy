pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local url_scheme = {
	request = function(flow)
		if flow.request.url.protocol == "http" then
			flow.request.url.protocol = "https"
		end
	end,
}
Extensions = { url_scheme }
