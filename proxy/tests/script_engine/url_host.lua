Extensions = {
	{
		request = function(flow)
			print("Original host: " .. flow.request.url.host)
			if flow.request.url.host == "localhost:1234" then
				flow.request.url.host = "example.com:4321"
			end
		end,
	},
}
