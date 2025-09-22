pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local notify = {
	request = function()
		Roxy.notify(1, "hi")
	end,

	response = function()
		Roxy.notify(2, "there")
	end,
}
Extensions = { notify }
