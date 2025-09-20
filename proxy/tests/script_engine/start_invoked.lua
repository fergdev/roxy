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
			flow.response.body.text = "" .. count
		end,
	},
}
