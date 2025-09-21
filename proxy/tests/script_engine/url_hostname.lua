Extensions = {
	{
		request = function(flow)
			if flow.request.url.hostname == "localhost" then
				flow.request.url.hostname = "example.com"
			end
		end,
	},
}
