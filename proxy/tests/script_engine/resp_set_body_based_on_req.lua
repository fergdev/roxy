Extensions = {
	{
		response = function(flow)
			if flow.request.url.host == "example.com" then
				flow.response.body.text = "intercepted"
			end
		end,
	},
}
