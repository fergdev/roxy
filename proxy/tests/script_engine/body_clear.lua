Extensions = {
	{
		request = function(flow)
			flow.request.body:clear()
		end,
		response = function(flow)
			flow.response.body:clear()
		end,
	},
}
