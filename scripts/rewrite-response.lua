function intercept_request(req)
	print("[lua] intercept_request to:", req.url)
	return req
end

function intercept_response(res)
	print("[lua] intercept_response")

	-- Overwrite the body with a custom HTML message
	res.body = "<html><body><h1>Intercepted by Roxy</h1><p>This response was rewritten.</p></body></html>"

	-- Add or override a response header
	res.headers["Content-Type"] = "text/html"
	res.headers["X-Intercepted-By"] = "Roxy"

	return res
end
