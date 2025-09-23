pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local url_scheme = {
	request = function(flow)
		if flow.request.url.protocol == Protocol.HTTP then
			flow.request.url.protocol = Protocol.HTTPS
		end
	end,
}
Extensions = { url_scheme }
