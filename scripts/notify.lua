local function intercept_request(flow)
	Roxy.notify("test! " .. flow.request.host, 0)
end

local function intercept_response(flow)
	Roxy.notify("test! " .. flow.response.status, 1)
end

Extensions = {
	{
		intercept_request,
		intercept_response,
	},
}
