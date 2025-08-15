Extensions = {
	{
		function(flow)
			print("[lua] intercept_request to:", flow.request.host)
			flow.request.host = "example.com" -- Change the host to example.com
		end,
		function(flow)
			print("[lua] intercept_response")
			flow.status = 404
		end,
	},
}
