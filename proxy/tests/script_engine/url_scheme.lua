Extensions = {
	{
		request = function(flow)
			if flow.request.url.scheme == "http" then
				flow.request.url.scheme = "https"
			end
		end,
	},
}
