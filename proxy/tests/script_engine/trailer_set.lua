Extensions = {
	{
		request = function(flow)
			flow.request.trailers:set("X-Trailer1", "request")
		end,
		response = function(flow)
			flow.response.trailers:set("X-Trailer1", "response")
		end,
	},
}
