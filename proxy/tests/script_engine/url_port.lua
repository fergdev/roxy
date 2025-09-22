Extensions = {
	{
		request = function(flow)
			if flow.request.url.port == 1234 then
				flow.request.url.port = 8080
			end
		end,
	},
}
