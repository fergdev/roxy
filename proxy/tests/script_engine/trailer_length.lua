local trailer_length = {
	request = function(flow)
		local t = flow.request.trailers
		if #t == 12 then
			t:clear()
		end
	end,
	response = function(flow)
		local t = flow.response.trailers
		if #t == 12 then
			t:clear()
		end
	end,
}
Extensions = { trailer_length }
