local out_file = ""
local count = 0
Extensions = {
	{
		start = function()
			count = 10
		end,
		request = function(flow)
			flow.request.body.text = "" .. count
			count = count + 1
		end,
		response = function(flow)
			out_file = flow.response.body.text
			flow.response.body.text = "" .. count
		end,
		stop = function()
			local f = assert(io.open(out_file, "w"))
			f:write(string.format('{"stopped":true,"count":%d}', count))
			f:close()
		end,
	},
}
