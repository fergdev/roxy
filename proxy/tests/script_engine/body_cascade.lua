local function req(flow)
	flow.request.body.text = flow.request.body.text .. " request1"
end

local function resp(flow)
	flow.response.body.text = flow.response.body.text .. " response1"
end
Extensions = {
	{
		request = req,
		response = resp,
	},
	{
		request = function(flow)
			flow.request.body.text = flow.request.body.text .. " request2"
		end,
		response = function(flow)
			flow.response.body.text = flow.response.body.text .. " response2"
		end,
	},
}
