local counter = 0
Extensions = {
	{
		request = function(flow)
			counter = counter + 1
		end,
		response = function(flow)
			counter = counter + 1
		end,
	},
}
