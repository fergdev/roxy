Extensions = {
	{
		request = function(flow)
			flow.request.headers:set("X-Header1", "request")
		end,
		response = function(flow)
			flow.response.headers:set("X-Header1", "response")
		end,
	},
}
