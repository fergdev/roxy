pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local void = {
	start = function() end,
	stop = function() end,
	request = function(flow) end,
	response = function(flow) end,
}
Extensions = { void }
