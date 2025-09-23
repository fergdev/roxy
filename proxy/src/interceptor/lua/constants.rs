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

    register_status_table(lua)?;
    Ok(())
}

pub fn register_status_table(lua: &Lua) -> LuaResult<()> {
    let status = lua.create_table()?;

    status.set("CONTINUE", 100)?;
    status.set("SWITCHING_PROTOCOLS", 101)?;
    status.set("PROCESSING", 102)?;
    status.set("OK", 200)?;
    status.set("CREATED", 201)?;
    status.set("ACCEPTED", 202)?;
    status.set("NON_AUTHORITATIVE_INFORMATION", 203)?;
    status.set("NO_CONTENT", 204)?;
    status.set("RESET_CONTENT", 205)?;
    status.set("PARTIAL_CONTENT", 206)?;
    status.set("MULTI_STATUS", 207)?;
    status.set("ALREADY_REPORTED", 208)?;
    status.set("IM_USED", 226)?;
    status.set("MULTIPLE_CHOICES", 300)?;
    status.set("MOVED_PERMANENTLY", 301)?;
    status.set("FOUND", 302)?;
    status.set("SEE_OTHER", 303)?;
    status.set("NOT_MODIFIED", 304)?;
    status.set("USE_PROXY", 305)?;
    status.set("TEMPORARY_REDIRECT", 307)?;
    status.set("PERMANENT_REDIRECT", 308)?;
    status.set("BAD_REQUEST", 400)?;
    status.set("UNAUTHORIZED", 401)?;
    status.set("PAYMENT_REQUIRED", 402)?;
    status.set("FORBIDDEN", 403)?;
    status.set("NOT_FOUND", 404)?;
    status.set("METHOD_NOT_ALLOWED", 405)?;
    status.set("NOT_ACCEPTABLE", 406)?;
    status.set("PROXY_AUTHENTICATION_REQUIRED", 407)?;
    status.set("REQUEST_TIMEOUT", 408)?;
    status.set("CONFLICT", 409)?;
    status.set("GONE", 410)?;
    status.set("LENGTH_REQUIRED", 411)?;
    status.set("PRECONDITION_FAILED", 412)?;
    status.set("PAYLOAD_TOO_LARGE", 413)?;
    status.set("URI_TOO_LONG", 414)?;
    status.set("UNSUPPORTED_MEDIA_TYPE", 415)?;
    status.set("RANGE_NOT_SATISFIABLE", 416)?;
    status.set("EXPECTATION_FAILED", 417)?;
    status.set("IM_A_TEAPOT", 418)?;
    status.set("MISDIRECTED_REQUEST", 421)?;
    status.set("UNPROCESSABLE_ENTITY", 422)?;
    status.set("LOCKED", 423)?;
    status.set("FAILED_DEPENDENCY", 424)?;
    status.set("TOO_EARLY", 425)?;
    status.set("UPGRADE_REQUIRED", 426)?;
    status.set("PRECONDITION_REQUIRED", 428)?;
    status.set("TOO_MANY_REQUESTS", 429)?;
    status.set("REQUEST_HEADER_FIELDS_TOO_LARGE", 431)?;
    status.set("UNAVAILABLE_FOR_LEGAL_REASONS", 451)?;
    status.set("INTERNAL_SERVER_ERROR", 500)?;
    status.set("NOT_IMPLEMENTED", 501)?;
    status.set("BAD_GATEWAY", 502)?;
    status.set("SERVICE_UNAVAILABLE", 503)?;
    status.set("GATEWAY_TIMEOUT", 504)?;
    status.set("HTTP_VERSION_NOT_SUPPORTED", 505)?;
    status.set("VARIANT_ALSO_NEGOTIATES", 506)?;
    status.set("INSUFFICIENT_STORAGE", 507)?;
    status.set("LOOP_DETECTED", 508)?;
    status.set("NOT_EXTENDED", 510)?;
    status.set("NETWORK_AUTHENTICATION_REQUIRED", 511)?;

    lua.globals().set("Status", status)?;
    Ok(())
}
