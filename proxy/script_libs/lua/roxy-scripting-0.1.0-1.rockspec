package = "roxy-scripting"
version = "0.1.0-1"
source = { url = "git+https://example.com/roxy/roxy-lua.git", tag = "v0.1.0" }
description = {
	summary = "Roxy scripting API types and shim",
	license = "MIT",
	homepage = "https://example.com/roxy",
}
dependencies = { "lua >= 5.1" }
build = {
	type = "none",
	install = {
		lua = {
			["roxy/init"] = "roxy/init.lua",
			["roxy/types"] = "roxy/types.lua",
		},
	},
}
