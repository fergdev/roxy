Extensions = {
	{
		request = function(flow)
			if flow.request.url.port == 80 then
				flow.request.url.port = 8080
			end
		end,
	},
}
