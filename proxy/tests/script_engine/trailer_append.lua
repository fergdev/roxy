Extensions = {
	{
		request = function(flow)
			flow.request.trailers:append("X-Trailer1", "request")
			flow.request.trailers:append("X-Trailer9", "request")
		end,
		response = function(flow)
			flow.response.trailers:append("X-Trailer1", "response")
			flow.response.trailers:append("X-Trailer9", "response")
		end,
	},
}
