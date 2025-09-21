Extensions = {
	{
		request = function(flow)
			if flow.request.headers:has("X-Header1") then
				flow.request.body.text = "has"
			end
		end,
		response = function(flow)
			if flow.response.headers:has("X-Header1") then
				flow.response.body.text = "has"
			end
		end,
	},
}
