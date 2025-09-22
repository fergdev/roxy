local trailer_to_string = {
	request = function(flow)
		flow.request.body.text = tostring(flow.request.trailers)
	end,
	response = function(flow)
		flow.response.body.text = tostring(flow.response.trailers)
	end,
}
Extensions = { trailer_to_string }
