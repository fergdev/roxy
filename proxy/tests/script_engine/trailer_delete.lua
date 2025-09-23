local trailer_delete = {

	request = function(flow)
		flow.request.trailers:delete("X-trailer1")
		flow.request.trailers["X-trailer2"] = nil
		flow.request.trailers["X-trailer3"] = nil -- TODO: there is no 3rd way to delete headers in lua
	end,
	response = function(flow)
		flow.response.trailers:delete("X-trailer1")
		flow.response.trailers["X-trailer2"] = nil
		flow.response.trailers["X-trailer3"] = nil -- TODO: there is no 3rd way to delete headers in lua
	end,
}
Extensions = { trailer_delete }
