pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local version_set = {
	request = function(flow)
		flow.request.version = Version.HTTP3
	end,
	response = function(flow)
		flow.response.version = Version.HTTP3
	end,
}
Extensions = { version_set }
