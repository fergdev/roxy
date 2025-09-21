Extensions = {
	{
		request = function(flow)
			if flow.request.url.authority == "dave:1234@localhost:1234" then
				flow.request.url.authority = "damo:abcd@localhost:4321"
			end
		end,
	},
}
