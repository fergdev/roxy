local function req(flow)
	flow.request.body.text = string.gsub(flow.request.body.text, "replaceme", "gone")
end

local function resp(flow)
	flow.response.body.text = string.gsub(flow.response.body.text, "to_go", "it_went")
end
Extensions = {
	{
		request = req,
		response = resp,
	},
}
