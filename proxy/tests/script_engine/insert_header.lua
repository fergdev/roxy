Extensions = {
	{
		request = function(flow)
			flow.request.headers:append("set-cookie", "test-request")
		end,
		response = function(flow)
			flow.response.headers:append("set-cookie", "test-response")
		end,
	},
}
