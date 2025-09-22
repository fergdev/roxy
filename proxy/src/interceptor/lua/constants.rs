use mlua::Lua;
use mlua::prelude::*;

pub fn register_constants(lua: &Lua) -> LuaResult<()> {
    let method = lua.create_table()?;
    method.set("CONNECT", "CONNECT")?;
    method.set("DELETE", "DELETE")?;
    method.set("GET", "GET")?;
    method.set("HEAD", "HEAD")?;
    method.set("OPTIONS", "OPTIONS")?;
    method.set("PATCH", "PATCH")?;
    method.set("POST", "POST")?;
    method.set("PUT", "PUT")?;
    method.set("TRACE", "TRACE")?;
    lua.globals().set("Method", method)?;

    let proto = lua.create_table()?;
    proto.set("HTTP", "http")?;
    proto.set("HTTPS", "https")?;
    lua.globals().set("Protocol", proto)?;

    let version = lua.create_table()?;
    version.set("HTTP09", "HTTP/0.9")?;
    version.set("HTTP10", "HTTP/1.0")?;
    version.set("HTTP11", "HTTP/1.1")?;
    version.set("HTTP2", "HTTP/2")?;
    version.set("HTTP3", "HTTP/3")?;
    lua.globals().set("Version", version)?;

    Ok(())
}
