Extensions = {
	{
		request = function(flow)
			if flow.request.url.username == "dave" then
				flow.request.url.username = "damo"
			end
		end,
	},
}
