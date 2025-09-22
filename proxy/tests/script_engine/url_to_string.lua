Extensions = {
	{
		request = function(flow)
			flow.request.body.text = tostring(flow.request.url)
		end,
	},
}
