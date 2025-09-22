Extensions = {
	{
		request = function(flow)
			if flow.request.url.search_params["foo"] == "bar" then
				flow.request.url.search_params:set("foo", "baz")
			end
		end,
	},
}
