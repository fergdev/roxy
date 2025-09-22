Extensions = {
	{
		request = function(flow)
			if flow.request.trailers:has("X-trailer1") then
				flow.request.body.text = "has"
			end
		end,
		response = function(flow)
			if flow.response.trailers:has("X-trailer1") then
				flow.response.body.text = "has"
			end
		end,
	},
}
