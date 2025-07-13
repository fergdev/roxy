function intercept_request(req)
	print("[lua] intercept_request to:", req.host)
	req.host = "example.com" -- Change the host to example.com

	return req
end

function intercept_response(res)
	print("[lua] intercept_response")

	return res
end
