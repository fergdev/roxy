pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local url_password = {
	request = function(flow)
		if flow.request.url.password == "1234" then
			flow.request.url.password = "abcd"
		end
	end,
}
Extensions = { url_password }
