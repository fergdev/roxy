Extensions = {
	{
		function(flow)
			flow.request.version = "HTTP/3.0"
		end,
		function(flow)
			flow.response.version = "HTTP/3.0"
		end,
	},
}
