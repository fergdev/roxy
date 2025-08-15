local function table_to_string(tbl, indent)
	indent = indent or 0
	local output = string.rep("  ", indent) .. "{\n"
	for k, v in pairs(tbl) do
		local key = tostring(k)
		local value
		if type(v) == "table" then
			value = table_to_string(v, indent + 1)
		else
			value = string.format("%q", tostring(v))
		end
		output = output .. string.rep("  ", indent + 1) .. "[" .. key .. "] = " .. value .. ",\n"
	end
	output = output .. string.rep("  ", indent) .. "}"
	return output
end

local function log_to_file(label, data)
	local f = io.open("intercept.log", "a")
	if f then
		f:write("==== " .. label .. " ====\n")
		f:write(table_to_string(data) .. "\n\n")
		f:close()
	else
		print("Could not open log file")
	end
end

local function intercept_request(req)
	log_to_file("Request", req)
end

local function intercept_response(res)
	log_to_file("Response", res)
end

Extensions = {
	{
		intercept_request,
		intercept_response,
	},
}
