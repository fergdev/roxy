Extensions = {
	{
		request = function(flow)
			if #flow.request.body == 10 then
				flow.request.body.text = "len is 10 request"
			end
		end,
		response = function(flow)
			if #flow.response.body == 10 then
				flow.response.body.text = "len is 10 response"
			end
		end,
	},
}
