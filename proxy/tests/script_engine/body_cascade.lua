pcall(require, "../../script_libs/lua/roxy.lua")

--- Append a numeric id to request/response bodies.
---@param id integer
---@return Extension
local function make_body_cascade(id)
	---@type Extension
	local ext = {
		---@param flow Flow
		request = function(flow)
			local t = flow.request.body.text or ""
			flow.request.body.text = t .. " request" .. tostring(id)
		end,

		---@param flow Flow
		response = function(flow)
			local t = flow.response.body.text or ""
			flow.response.body.text = t .. " response" .. tostring(id)
		end,
	}
	return ext
end

Extensions = {
	make_body_cascade(1),
	make_body_cascade(2),
}
