Extensions = {
	{
		request = function(flow)
			flow.request.trailers:clear()
		end,
		response = function(flow)
			flow.response.trailers:clear()
		end,
	},
}
