Extensions = {
	{
		request = function(flow)
			if flow.request.url.path == "/some/path" then
				flow.request.url.path = "/another/path"
			end
		end,
	},
}
