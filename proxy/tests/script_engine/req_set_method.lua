Extensions = {
	{
		request = function(flow)
			if flow.request.method == "GET" then
				flow.request.method = "POST"
			end
		end,
	},
}
