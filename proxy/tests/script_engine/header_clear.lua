Extensions = {
	{
		request = function(flow)
			flow.request.headers:clear()
		end,
		response = function(flow)
			flow.response.headers:clear()
		end,
	},
}
