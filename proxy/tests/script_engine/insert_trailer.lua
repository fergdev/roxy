Extensions = {
	{
		request = function(flow)
			flow.request.trailers:append("set-cookie", "test-request")
		end,
		response = function(flow)
			flow.response.trailers:append("set-cookie", "test-response")
		end,
	},
}
