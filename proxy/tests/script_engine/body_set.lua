Extensions = {
	{
		request = function(flow)
			flow.request.body.text = "rewrite request"
		end,
		response = function(flow)
			flow.response.body.text = "rewrite response"
		end,
	},
}
