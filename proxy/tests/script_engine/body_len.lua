Extensions = {
	{
		request = function(flow)
			local len = flow.request.body:len()
			print("Request body length:" .. len)
			if len == 10 then
				flow.request.body.text = "len is 10 request"
			end
		end,
		response = function(flow)
			local len = flow.response.body:len()
			print("Response body length:", len)
			if len == 10 then
				flow.response.body.text = "len is 10 response"
			end
		end,
	},
}
