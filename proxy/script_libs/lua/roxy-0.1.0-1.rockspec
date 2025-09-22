package = "roxy"
version = "0.1.0-1"
source = {
	url = "git+https://github.com/fergdev/roxy.git",
	tag = "main",
}
description = {
	summary = "A MITM proxy written in Rust with Lua API",
	detailed = [[
      roxy-proxy is a programmable MITM proxy, exposing Lua APIs
      for intercepting requests and responses.
   ]],
	homepage = "https://github.com/fergdev/roxy",
	license = "MIT",
}
dependencies = {
	"lua >= 5.3",
}
build = {
	type = "builtin",
	modules = {
		["roxy_proxy"] = "roxy.lua",
	},
}
