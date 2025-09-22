Extensions = {
	{
		request = function(flow)
			if #flow.request.headers == 12 then
				flow.request.headers:clear()
			end
		end,
		response = function(flow)
			if #flow.response.headers == 12 then
				flow.response.headers:clear()
			end
		end,
	},
}
