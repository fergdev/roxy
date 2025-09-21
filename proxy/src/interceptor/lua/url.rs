use std::str::FromStr;
use std::sync::{Arc, Mutex};

use mlua::UserData;
use mlua::Value;
use mlua::prelude::*;
use roxy_shared::uri::RUri;
use tracing::info;
use url::Url;

use crate::interceptor::KEY_AUTHORITY;
use crate::interceptor::KEY_HOST;
use crate::interceptor::KEY_HOSTNAME;
use crate::interceptor::KEY_PASSWORD;
use crate::interceptor::KEY_PATH;
use crate::interceptor::KEY_PORT;
use crate::interceptor::KEY_SCHEME;
use crate::interceptor::KEY_USERNAME;
use crate::interceptor::lua::query::LuaQueryView;
use crate::interceptor::lua::util::KEY_NEW;

#[derive(Clone, Debug)]
pub struct LuaUrl {
    uri: Arc<Mutex<Url>>,
}

impl LuaUrl {
    #[allow(clippy::unwrap_used)]
    pub fn from_ruri(uri: RUri) -> Self {
        let url =
            Url::parse(&uri.to_string()).unwrap_or_else(|_| Url::parse("http://invalid/").unwrap());
        Self {
            uri: Arc::new(Mutex::new(url)),
        }
    }

    pub fn to_ruri(&self) -> LuaResult<RUri> {
        let u = self
            .uri
            .lock()
            .map_err(|e| LuaError::external(format!("poinsoned: {e}")))?;
        RUri::from_str(u.as_str()).map_err(|e| LuaError::external(format!("invalid URL: {e}")))
    }

    fn lock(&self) -> LuaResult<std::sync::MutexGuard<'_, Url>> {
        self.uri
            .lock()
            .map_err(|e| LuaError::external(format!("lock poisoned: {e}")))
    }

    fn parse(&self) -> LuaResult<Url> {
        let u = self
            .uri
            .lock()
            .map_err(|e| LuaError::external(e.to_string()))?;
        Ok(u.clone())
    }

    fn write_from(&self, parsed: &Url) -> LuaResult<()> {
        let mut guard = self
            .uri
            .lock()
            .map_err(|e| LuaError::external(e.to_string()))?;
        *guard = parsed.clone();
        Ok(())
    }

    fn get_href(&self) -> LuaResult<String> {
        Ok(self.parse()?.as_str().to_string())
    }
    fn set_href(&self, href: &str) -> LuaResult<()> {
        let parsed = Url::parse(href).map_err(|e| LuaError::external(e.to_string()))?;
        self.write_from(&parsed)
    }

    fn get_scheme(&self) -> LuaResult<String> {
        let guard = self.lock()?;
        Ok(guard.scheme().to_owned())
    }
    fn set_scheme(&self, proto_with_colon: &str) -> LuaResult<()> {
        let mut u = self.parse()?;
        let p = proto_with_colon
            .strip_suffix(':')
            .unwrap_or(proto_with_colon);
        u.set_scheme(p)
            .map_err(|_| LuaError::external("invalid protocol"))?;
        self.write_from(&u)
    }

    fn get_username(&self) -> LuaResult<String> {
        Ok(self.parse()?.username().to_string())
    }
    fn set_username(&self, user: &str) -> LuaResult<()> {
        let mut u = self.parse()?;
        u.set_username(user)
            .map_err(|_| LuaError::external("invalid username"))?;
        self.write_from(&u)
    }

    fn get_password(&self) -> LuaResult<String> {
        Ok(self.parse()?.password().unwrap_or("").to_string())
    }
    fn set_password(&self, pass: &str) -> LuaResult<()> {
        let mut u = self.parse()?;
        u.set_password(Some(pass))
            .map_err(|_| LuaError::external("invalid password"))?;
        self.write_from(&u)
    }

    fn get_authority(&self) -> LuaResult<String> {
        Ok(self.parse()?.authority().to_string())
    }

    fn set_authority(&self, authority: &str) -> LuaResult<()> {
        info!("set_authority: {authority}");
        let mut u = self.parse()?;
        if authority.contains('@') {
            let mut split = authority.split('@');
            let user = split.next().ok_or(LuaError::external("Missing username"))?;
            let host = split.next().ok_or(LuaError::external("Missing password"))?;
            let mut user = user.split(':');
            let username = user.next().unwrap_or("");
            let password = user.next().unwrap_or("");

            let mut host = host.split(':');
            let hostname = host.next().unwrap_or("");
            let port = host.next().unwrap_or("");

            u.set_username(username)
                .map_err(|_| LuaError::external("invalid username"))?;
            u.set_password(Some(password))
                .map_err(|_| LuaError::external("invalid password"))?;
            u.set_host(Some(hostname))
                .map_err(|_| LuaError::external("invalid host"))?;
            u.set_port(if port.is_empty() {
                None
            } else {
                Some(
                    port.parse::<u16>()
                        .map_err(|_| LuaError::external("bad port"))?,
                )
            })
            .map_err(|_| LuaError::external("bad port"))?;
        } else {
            let mut host = authority.split(':');
            let hostname = host.next().unwrap_or("");
            let port = host.next().unwrap_or("");
            u.set_host(Some(hostname))
                .map_err(|_| LuaError::external("invalid host"))?;
            u.set_port(if port.is_empty() {
                None
            } else {
                Some(
                    port.parse::<u16>()
                        .map_err(|_| LuaError::external("bad port"))?,
                )
            })
            .map_err(|_| LuaError::external("bad port"))?;
        }
        self.write_from(&u)?;
        Ok(())
    }

    fn get_port(&self) -> LuaResult<u16> {
        Ok(self.parse()?.port_or_known_default().unwrap_or_default())
    }
    fn set_port(&self, port: u16) -> LuaResult<()> {
        let mut u = self.parse()?;
        u.set_port(Some(port))
            .map_err(|_| LuaError::external("bad port"))?;
        self.write_from(&u)
    }

    fn get_host(&self) -> LuaResult<String> {
        let u = self.parse()?;
        Ok(url::quirks::host(&u).to_string())
    }
    fn set_host(&self, host_port: &str) -> LuaResult<()> {
        let mut u = self.parse()?;
        if let Some((h, pstr)) = host_port.rsplit_once(':') {
            if let Ok(p) = pstr.parse::<u16>() {
                u.set_host(Some(h))
                    .map_err(|_| LuaError::external("invalid host"))?;
                u.set_port(Some(p))
                    .map_err(|_| LuaError::external("bad port"))?;
            } else {
                u.set_host(Some(host_port))
                    .map_err(|_| LuaError::external("invalid host"))?;
                u.set_port(None).ok();
            }
        } else {
            u.set_host(Some(host_port))
                .map_err(|_| LuaError::external("invalid host"))?;
            u.set_port(None).ok();
        }
        self.write_from(&u)
    }

    fn get_hostname(&self) -> LuaResult<String> {
        let u = self.parse()?;
        Ok(url::quirks::hostname(&u).to_string())
    }
    fn set_hostname(&self, hostname: &str) -> LuaResult<()> {
        let mut u = self.parse()?;
        url::quirks::set_hostname(&mut u, hostname)
            .map_err(|_| LuaError::external("invalid hostname"))?;
        self.write_from(&u)
    }

    fn get_path(&self) -> LuaResult<String> {
        Ok(self.parse()?.path().to_string())
    }
    fn set_path(&self, path: &str) -> LuaResult<()> {
        let mut u = self.parse()?;
        u.set_path(path);
        self.write_from(&u)
    }

    fn get_search(&self) -> LuaResult<String> {
        Ok(self
            .parse()?
            .query()
            .map(|q| format!("?{q}"))
            .unwrap_or_default())
    }
    fn set_search(&self, search: &str) -> LuaResult<()> {
        let mut u = self.parse()?;
        let s = search.strip_prefix('?').unwrap_or(search);
        if s.is_empty() {
            u.set_query(None);
        } else {
            u.set_query(Some(s));
        }
        self.write_from(&u)
    }

    fn get_origin(&self) -> LuaResult<String> {
        let u = self.parse()?;
        let scheme = u.scheme();
        let host = u.host_str().unwrap_or("");
        let port = u.port();
        Ok(match port {
            Some(p) => format!("{scheme}://{host}:{p}"),
            None => format!("{scheme}://{host}"),
        })
    }
}

impl UserData for LuaUrl {
    fn add_methods<M: LuaUserDataMethods<Self>>(m: &mut M) {
        m.add_method("to_string", |_, this, ()| this.get_href());

        m.add_meta_method(LuaMetaMethod::Index, |lua, this, key: Value| {
            let s = match key {
                Value::String(s) => s,
                _ => return Ok(Value::Nil),
            };
            let k = s.to_str()?;
            let out = match &*k {
                "href" => Value::String(lua.create_string(&this.get_href()?)?),
                KEY_SCHEME => Value::String(lua.create_string(&this.get_scheme()?)?),
                KEY_USERNAME => Value::String(lua.create_string(&this.get_username()?)?),
                KEY_PASSWORD => Value::String(lua.create_string(&this.get_password()?)?),
                KEY_HOST => Value::String(lua.create_string(&this.get_host()?)?),
                KEY_HOSTNAME => Value::String(lua.create_string(&this.get_hostname()?)?),
                KEY_PORT => Value::Integer(this.get_port()? as i64),
                KEY_PATH => Value::String(lua.create_string(&this.get_path()?)?),
                KEY_AUTHORITY => Value::String(lua.create_string(&this.get_authority()?)?),
                "search" => Value::String(lua.create_string(&this.get_search()?)?),
                "origin" => Value::String(lua.create_string(&this.get_origin()?)?),
                "searchParams" => {
                    let ud = lua.create_userdata(LuaQueryView {
                        uri: this.uri.clone(),
                    })?;
                    Value::UserData(ud)
                }
                "toString" => {
                    let ud = lua.create_userdata(this.clone())?;
                    let f: LuaFunction = ud.get("to_string")?;
                    Value::Function(f)
                }
                _ => Value::Nil,
            };
            Ok(out)
        });

        m.add_meta_method_mut(
            LuaMetaMethod::NewIndex,
            |_, this, (key, val): (Value, Value)| {
                let k = match key {
                    Value::String(s) => s.to_str()?.to_string(),
                    _ => return Err(LuaError::external("property name must be string")),
                };
                match (k.as_str(), val) {
                    ("href", Value::String(s)) => this.set_href(s.to_str()?.as_ref())?,
                    (KEY_SCHEME, Value::String(s)) => this.set_scheme(s.to_str()?.as_ref())?,
                    (KEY_USERNAME, Value::String(s)) => this.set_username(s.to_str()?.as_ref())?,
                    (KEY_PASSWORD, Value::String(s)) => this.set_password(s.to_str()?.as_ref())?,
                    (KEY_AUTHORITY, Value::String(s)) => {
                        this.set_authority(s.to_str()?.as_ref())?
                    }
                    (KEY_HOST, Value::String(s)) => this.set_host(s.to_str()?.as_ref())?,
                    (KEY_HOSTNAME, Value::String(s)) => this.set_hostname(s.to_str()?.as_ref())?,
                    (KEY_PORT, Value::Integer(s)) => this.set_port(s as u16)?,
                    (KEY_PATH, Value::String(s)) => this.set_path(s.to_str()?.as_ref())?,
                    ("search", Value::String(s)) => this.set_search(s.to_str()?.as_ref())?,
                    ("origin", _) => return Err(LuaError::external("origin is read-only")),
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

        m.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| this.get_href());
    }
}

pub(crate) fn register_url(lua: &Lua) -> LuaResult<LuaTable> {
    let tbl = lua.create_table()?;

    let new = lua.create_function(|_, href: Option<String>| {
        let ruri = match href {
            Some(s) => RUri::from_str(&s)
                .map_err(|e| LuaError::external(format!("invalid URL '{}': {e}", s)))?,
            None => RUri::default(),
        };
        Ok(LuaUrl::from_ruri(ruri))
    })?;

    tbl.set(KEY_NEW, new)?;
    lua.globals().set("Url", tbl.clone())?;
    Ok(tbl)
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::interceptor::lua::engine::register_functions;

    use mlua::prelude::*;

    fn with_lua<F: FnOnce(&Lua) -> LuaResult<()>>(f: F) {
        let lua = Lua::new();
        register_functions(&lua, None).expect("register functions");
        f(&lua).expect("lua ok");
    }

    #[test]
    fn u01_construct_and_getters() {
        with_lua(|lua| {
            lua.load(
                r#"
                print("u01_construct_and_getters")
                local u = Url.new("https://user:pass@example.com:8443/a/b?x=1")
                assert(u.href == "https://user:pass@example.com:8443/a/b?x=1")
                print("scheme:", u.scheme)
                assert(u.scheme == "https")
                print("authority:", u.authority)
                assert(u.authority == "user:pass@example.com:8443")
                assert(u.username == "user")
                assert(u.password == "pass")
                print("host:", u.host)
                assert(u.host == "example.com:8443")
                assert(u.hostname == "example.com")
                assert(u.port == 8443)
                assert(u.path == "/a/b")
                assert(u.search == "?x=1")
                assert(u.origin == "https://example.com:8443")
                assert(tostring(u) == u.href)
                print("u01_construct_and_getters done")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn u02_setters_roundtrip() {
        with_lua(|lua| {
            lua.load(
                r#"
                local u = Url.new("http://x/")
                u.scheme   = "https"
                u.username = "me"
                u.password = "pw"
                u.host     = "example.org:444"
                u.port     = 444
                u.path     = "/p/q"
                u.search   = "?a=1"
                -- verify
                assert(u.scheme == "https")
                assert(u.username == "me")
                assert(u.password == "pw")
                assert(u.host == "example.org:444")
                assert(u.port == 444)
                assert(u.path == "/p/q")
                assert(u.search == "?a=1")
                local h = u.href
                assert(h:find("^https://me:pw@example.org:444/p/q%?a=1") ~= nil)
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn u03_search_params_set_append_delete_clear() {
        with_lua(|lua| {
            lua.load(
                r#"
                local u = Url.new("http://x/?a=1&a=2&b=3")
                local q = u.searchParams
                -- get / get_all / has
                assert(q:get("a") == "1")
                local all = q:get_all("a"); assert(#all == 2 and all[1] == "1" and all[2] == "2")
                assert(q:has("b") == true)

                -- append & set
                q:append("a", "9")
                q:set("b", "x")
                local s = q:to_string()
                -- membership checks (order not guaranteed)
                assert(s:find("a=1") and s:find("a=2") and s:find("a=9") and s:find("b=x"))

                -- delete
                q:delete("a")
                assert(q:has("a") == false)

                -- clear empties query
                q:clear()
                assert(q:to_string() ~= nil)
                assert(u.search ~= nil)
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn u04_origin_readonly_errors() {
        with_lua(|lua| {
            lua.load(
                r#"
            local u = Url.new("http://x/")
            local ok, err = pcall(function()
                u.origin = "http://nope/"
            end)
            assert(ok == false, "expected assigning origin to fail")
            assert(err ~= nil, "expected an error object/message")
        "#,
            )
            .exec()
        });
    }

    #[test]
    fn u05_tostring_method_and_property() {
        with_lua(|lua| {
            lua.load(
                r#"
                local u = Url.new("http://ex/p?q=1")
                -- to_string method exposed as toString accessor
                assert(u:to_string() == "http://ex/p?q=1")
                local sfn = u.toString
                assert(type(sfn) == "function")
                assert(sfn(u) == "http://ex/p?q=1")
                assert(tostring(u) == "http://ex/p?q=1")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn u06_setting_href_reserializes_parts() {
        with_lua(|lua| {
            lua.load(
                r#"
                local u = Url.new()
                u.href = "https://a.example/alpha?z=9"
                assert(u.scheme == "https")
                assert(u.host:match("^a%.example:?%d*$"))
                assert(u.path == "/alpha")
                assert(u.search == "?z=9")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn u07_set_host_parses_optional_port() {
        with_lua(|lua| {
            lua.load(
                r#"
                local u = Url.new("http://x/")
                u.host = "example.com"
                u.port = 1234
                assert(u.host == "example.com:1234")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn u08_search_setter_accepts_without_question_mark() {
        with_lua(|lua| {
            lua.load(
                r#"
                local u = Url.new("http://x/")
                u.search = "k=v"
                assert(u.search == "?k=v")
                u.search = ""
                assert(u.search == "")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn u09_query_view_bracket_sugar_and_coercion() {
        with_lua(|lua| {
            lua.load(
                r#"
                local u = Url.new("http://x/?n=1")
                local q = u.searchParams
                -- bracket sugar read
                assert(q["n"] == "1")
                -- numeric & boolean coercion on write
                q["n"] = 42
                q["t"] = true
                q["f"] = false
                local s = q:to_string()
                assert(s:find("n=42") and s:find("t=true") and s:find("f=false"))
                -- delete via nil
                q["n"] = nil
                assert(q:has("n") == false)
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn u10_invalid_href_errors() {
        with_lua(|lua| {
            lua.load(
                r#"
            local u = Url.new()
            local ok, err = pcall(function()
                u.href = "http://exa mple.com/"  -- space is invalid
            end)
            assert(ok == false, "expected setting invalid href to fail")
            assert(err ~= nil, "expected an error object/message")
        "#,
            )
            .exec()
        });
    }
}
