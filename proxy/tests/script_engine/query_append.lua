Extensions = {
	{
		request = function(flow)
			if flow.request.url.search_params["foo"] == "bar" then
				flow.request.url.search_params:append("foo", "baz")
			end
		end,
	},
}
