pcall(require, "../../script_libs/lua/roxy.lua")
---@type Extension
local header_length = {
	request = function(flow)
		if #flow.request.headers == 12 then
			flow.request.headers:clear()
		end
	end,
	response = function(flow)
		if #flow.response.headers == 12 then
			flow.response.headers:clear()
		end
	end,
}
Extensions = { header_length }
