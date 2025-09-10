local function req(flow)
	Roxy.notify(1, "hi")
end

local function resp(flow)
	Roxy.notify(2, "there")
end
Extensions = {
	{
		request = req,
		response = resp,
	},
}
