Extensions = {
	{
		request = function(flow)
			if flow.request.url.password == "1234" then
				flow.request.url.password = "abcd"
			end
		end,
	},
}
