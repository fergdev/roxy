local function intercept_request(flow)
	print("[lua] intercept_request to:", flow.request.url_pretty)
end

local function intercept_response(flow)
	print("[lua] intercept_response")

	-- Overwrite the body with a custom HTML message
	flow.response.body = "<html><body><h1>Intercepted by Roxy</h1><p>This response was rewritten.</p></body></html>"

	-- Add or override a response header
	flow.response.headers["Content-Type"] = "text/html"
	flow.response.headers["X-Intercepted-By"] = "Roxy"
end

Extensions = {
	{
		intercept_request,
		intercept_response,
	},
}
