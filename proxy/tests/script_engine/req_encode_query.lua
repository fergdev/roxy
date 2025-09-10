Extensions = {
	{
		request = function(flow)
			flow.request.url.searchParams["foo"] = "bar & baz"
			flow.request.url.searchParams["saison"] = "Été+hiver"
		end,
	},
}
