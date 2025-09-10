Extensions = {
	{
		request = function(flow)
			flow.request.url.searchParams["foo"] = "bar"
			flow.request.url.searchParams["a"] = "b"
			flow.request.url.searchParams["no"] = nil
			flow.request.url.searchParams["yes"] = nil
			flow.request.url.searchParams["saison"] = nil
		end,
	},
}
