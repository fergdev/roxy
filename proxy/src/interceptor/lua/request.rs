use std::sync::{Arc, Mutex, MutexGuard};

use cow_utils::CowUtils;
use http::Method;
use mlua::prelude::*;
use roxy_shared::{uri::RUri, version::HttpVersion};

use crate::{
    flow::InterceptedRequest,
    interceptor::{
        KEY_BODY, KEY_HEADERS, KEY_METHOD, KEY_TRAILERS, KEY_URL, KEY_VERSION,
        lua::{body::LuaBody, headers::LuaHeaders, url::LuaUrl, util::KEY_NEW},
    },
};

#[derive(Clone, Debug)]
pub(crate) struct LuaRequest {
    inner: Arc<Mutex<InterceptedRequest>>,
    pub uri: LuaUrl,
    pub headers: LuaHeaders,
    pub trailers: LuaHeaders,
    pub body: LuaBody,
}

impl Default for LuaRequest {
    fn default() -> Self {
        let inner = Arc::new(Mutex::new(InterceptedRequest::default()));
        let uri = LuaUrl::from_ruri(RUri::default());
        let headers = LuaHeaders::default();
        let trailers = LuaHeaders::default();
        let body = LuaBody::default();
        Self {
            inner,
            uri,
            headers,
            trailers,
            body,
        }
    }
}

impl LuaRequest {
    pub fn from_parts(inner: Arc<Mutex<InterceptedRequest>>) -> LuaResult<Self> {
        let (uri, headers, trailers, body) = {
            let g = inner
                .lock()
                .map_err(|e| LuaError::external(format!("lock poisoned: {e}")))?;
            (
                g.uri.clone(),
                g.headers.clone(),
                g.trailers.clone(),
                g.body.clone(),
            )
        };

        Ok(Self {
            inner,
            uri: LuaUrl::from_ruri(uri),
            headers: LuaHeaders::new(headers),
            trailers: LuaHeaders::new(trailers.unwrap_or_default()),
            body: LuaBody::from_bytes(body),
        })
    }
    fn lock(&self) -> LuaResult<MutexGuard<'_, InterceptedRequest>> {
        self.inner
            .lock()
            .map_err(|e| LuaError::external(format!("lock poisoned: {e}")))
    }
}

impl LuaUserData for LuaRequest {
    fn add_methods<M: LuaUserDataMethods<Self>>(m: &mut M) {
        m.add_meta_method(LuaMetaMethod::Index, |lua, this, key: LuaValue| {
            if let LuaValue::String(s) = key {
                let k = s.to_str()?;
                match &*k {
                    KEY_METHOD => {
                        let guard = this.lock()?;
                        let m = guard.method.to_string();
                        let lua_str = lua.create_string(&m)?;
                        return Ok(LuaValue::String(lua_str));
                    }
                    KEY_VERSION => {
                        let guard = this.lock()?;
                        let m = format!("{:?}", guard.version);
                        let lua_str = lua.create_string(&m)?;
                        return Ok(LuaValue::String(lua_str));
                    }
                    KEY_URL => {
                        let ud = lua.create_userdata(this.uri.clone())?;
                        return Ok(LuaValue::UserData(ud));
                    }
                    KEY_BODY => {
                        let ud = lua.create_userdata(this.body.clone())?;
                        return Ok(LuaValue::UserData(ud));
                    }
                    KEY_HEADERS => {
                        let ud = lua.create_userdata(this.headers.clone())?;
                        return Ok(LuaValue::UserData(ud));
                    }
                    KEY_TRAILERS => {
                        let ud = lua.create_userdata(this.trailers.clone())?;
                        return Ok(LuaValue::UserData(ud));
                    }
                    _ => {}
                }
            }
            Ok(LuaValue::Nil)
        });
        m.add_meta_method_mut(
            LuaMetaMethod::NewIndex,
            |_, this, (key, val): (LuaValue, LuaValue)| {
                let k = match key {
                    LuaValue::String(s) => s.to_str()?.to_string(),
                    _ => return Err(LuaError::external("property name must be string")),
                };

                match (k.as_str(), val) {
                    (KEY_METHOD, LuaValue::String(s)) => {
                        let mut g = this.lock()?;
                        let s = &*s.to_str()?;
                        let s = CowUtils::cow_to_uppercase(s);
                        g.method = Method::from_bytes(s.as_bytes())
                            .map_err(|e| LuaError::RuntimeError(format!("{e}")))?;
                    }
                    (KEY_VERSION, LuaValue::String(s)) => {
                        let value = &*s.to_str()?;
                        let version: HttpVersion = value.parse().map_err(|_| {
                            LuaError::RuntimeError(format!("invalid HTTP version '{}'", value))
                        })?;
                        let mut g = this.lock()?;
                        g.version = version;
                    }
                    (KEY_URL | KEY_HEADERS | KEY_TRAILERS | KEY_BODY, _) => {
                        return Err(LuaError::external(
                            "property is read-only; mutate its fields instead",
                        ));
                    }

                    _ => {
                        return Err(LuaError::external(format!(
                            "unsupported assignment to {}",
                            k
                        )));
                    }
                }
                Ok(())
            },
        );
        // TODO: implement
        // m.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| this.to_string());
    }
}

pub fn register_request(lua: &Lua) -> LuaResult<LuaTable> {
    let tbl = lua.create_table()?;
    let new = lua.create_function(|_, ()| {
        let inner = InterceptedRequest::default();
        Ok(LuaRequest::from_parts(Arc::new(Mutex::new(inner))))
    })?;
    tbl.set(KEY_NEW, new)?;
    lua.globals().set("Request", tbl.clone())?;
    Ok(tbl)
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::interceptor::lua::tests::with_lua;

    #[test]
    fn r01_constructor_and_types() {
        with_lua(|lua| {
            lua.load(
                r#"
                local req = Request.new()
                assert(req.method ~= nil, "method should exist")
                assert(req.version ~= nil, "version should exist")
                assert(req.url ~= nil, "url should exist")
                assert(req.headers ~= nil, "headers should exist")
                assert(req.trailers ~= nil, "trailers should exist")
                assert(req.body ~= nil, "body should exist")

                assert(type(req.method) == "string")
                assert(type(req.version) == "string")
                assert(type(req.url) == "userdata")
                assert(type(req.headers) == "userdata")
                assert(type(req.trailers) == "userdata")
                assert(type(req.body) == "userdata")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn r02_method_set_get_uppercases() {
        with_lua(|lua| {
            lua.load(
                r#"
                local req = Request.new()
                req.method = "post"
                assert(req.method == "POST", "http::Method should normalize to POST")
                req.method = "GET"
                assert(req.method == "GET")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn r03_version_set_valid_and_invalid() {
        with_lua(|lua| {
            lua.load(
                r#"
                local req = Request.new()
                -- valid
                req.version = "HTTP/2.0"
                -- we don't assert the getter formatting because implementation prints Debug,
                -- but at least setting shouldn't error for valid values.

                -- invalid
                local ok, err = pcall(function()
                    req.version = "HTTP/9.9"
                end)
                assert(ok == false and err ~= nil, "setting invalid version should error")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn r04_body_text_roundtrip_and_raw() {
        with_lua(|lua| {
            lua.load(
                r#"
                local req = Request.new()

                assert(req.body.is_empty == true)
                assert(#req.body == 0)

                req.body.text = "hello\x00world"
                assert(req.body.text == "hello\x00world")
                assert(req.body.is_empty == false)
                assert(#req.body == 11)

                local raw = req.body.raw
                assert(type(raw) == "string" and #raw == 11)

                req.body.raw = "\1\2\3"
                assert(#req.body == 3)
                assert(req.body.raw == "\1\2\3")

                req.body.text = "abc"
                assert(req.body.text == "abc")
                assert(#req.body == 3)
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn r05_headers_and_trailers_userdata_basics() {
        with_lua(|lua| {
            lua.load(
                r#"
                local req = Request.new()
                local h = req.headers
                h:set("Host", "example.com")
                assert(h:get("host") == "example.com")

                local t = req.trailers
                t:set("X-Foo", "bar")
                assert(t:get("x-foo") == "bar")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn r06_url_userdata_roundtrip() {
        with_lua(|lua| {
            lua.load(
                r#"
                local req = Request.new()
                local u = req.url
                u.scheme = "https:"
                u.host = "example.org"
                u.path = "/a/b"
                u.search = "?x=1&x=2"
                local href = tostring(u)
                assert(href:find("^https://example%.org/") ~= nil)
                assert(href:find("x=1") ~= nil and href:find("x=2") ~= nil)
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn r07_readonly_fields_enforced() {
        with_lua(|lua| {
            lua.load(
                r#"
                local req = Request.new()

                local ok1, err1 = pcall(function()
                    req.url = "https://evil.invalid/"
                end)
                assert(ok1 == false and err1 ~= nil)

                local ok2, err2 = pcall(function()
                    req.headers = {}
                end)
                assert(ok2 == false and err2 ~= nil)

                local ok3, err3 = pcall(function()
                    req.trailers = {}
                end)
                assert(ok3 == false and err3 ~= nil)

                local ok4, err4 = pcall(function()
                    req.body = "nope"
                end)
                assert(ok4 == false and err4 ~= nil)
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn r08_construct_and_mutate_realistically() {
        with_lua(|lua| {
            lua.load(
                r#"
                local req = Request.new()

                -- change method/version
                req.method = "PUT"
                req.version = "HTTP/1.1"  -- valid

                -- set some headers
                req.headers:set("Content-Type", "text/plain")
                req.headers:append("Set-Cookie", "a=1")
                req.headers:append("Set-Cookie", "b=2")
                local cookies = req.headers:get_all("set-cookie")
                assert(#cookies == 2)

                -- modify body
                req.body.text = "payload"
                assert(req.body.text == "payload")

                -- tweak URL query
                local q = req.url.search_params
                q:set("x", "1")
                q:append("x", "2")
                local qs = q:to_string()
                assert(qs:find("x=1") ~= nil and qs:find("x=2") ~= nil)
            "#,
            )
            .exec()
        });
    }
}
