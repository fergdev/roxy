Extensions = {
	{
		request = function(flow)
			flow.request.body.text = tostring(flow.request.headers)
		end,
		response = function(flow)
			flow.response.body.text = tostring(flow.response.headers)
		end,
	},
}
