pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local url_username = {
	request = function(flow)
		if flow.request.url.username == "dave" then
			flow.request.url.username = "damo"
		end
	end,
}
Extensions = { url_username }
