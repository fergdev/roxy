Extensions = {
	{
		request = function(flow)
			flow.request.headers:append("X-Header1", "request")
			flow.request.headers:append("X-Header9", "request")
		end,
		response = function(flow)
			flow.response.headers:append("X-Header1", "response")
			flow.response.headers:append("X-Header9", "response")
		end,
	},
}
