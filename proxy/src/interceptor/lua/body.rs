use std::sync::{Arc, Mutex, MutexGuard};

use bytes::Bytes;
use mlua::prelude::*;
use tracing::error;

use crate::interceptor::lua::util::KEY_NEW;

#[derive(Clone, Debug)]
pub(crate) struct LuaBody {
    pub(crate) inner: Arc<Mutex<Bytes>>,
}

impl Default for LuaBody {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Bytes::new())),
        }
    }
}

impl LuaBody {
    pub(crate) fn from_bytes(bytes: Bytes) -> Self {
        Self {
            inner: Arc::new(Mutex::new(bytes)),
        }
    }

    fn get_text(&self) -> LuaResult<String> {
        let g = self.lock()?;
        String::from_utf8(g.to_vec()).map_err(|e| LuaError::external(format!("invalid UTF-8: {e}")))
    }
    fn set_text(&mut self, s: &str) -> LuaResult<()> {
        let mut g = self.lock()?;
        *g = Bytes::from(s.as_bytes().to_vec());
        Ok(())
    }

    fn get_raw(&self, lua: &Lua) -> LuaResult<LuaString> {
        let g = self.lock()?;
        lua.create_string(g.as_ref())
    }

    fn set_raw(&mut self, b: &[u8]) -> LuaResult<()> {
        let mut g = self.lock()?;
        *g = Bytes::from(b.to_vec());
        Ok(())
    }

    fn len(&self) -> usize {
        match self.lock() {
            Ok(g) => g.len(),
            Err(e) => {
                error!("body lock error: {e}");
                0
            }
        }
    }
    fn is_empty(&self) -> bool {
        match self.lock() {
            Ok(g) => g.is_empty(),
            Err(e) => {
                error!("body lock error: {e}");
                true
            }
        }
    }
    fn clear(&self) -> LuaResult<()> {
        let mut g = self.lock()?;
        *g = Bytes::new();
        Ok(())
    }
    fn lock(&self) -> LuaResult<MutexGuard<'_, Bytes>> {
        self.inner
            .lock()
            .map_err(|e| LuaError::external(format!("lock poisoned: {e}")))
    }
}

impl LuaUserData for LuaBody {
    fn add_methods<M: LuaUserDataMethods<Self>>(m: &mut M) {
        m.add_method("get_text", |_, this, ()| this.get_text());
        m.add_method_mut("set_text", |_, this, s: String| this.set_text(&s));

        m.add_method("get_raw", |lua, this, ()| this.get_raw(lua));
        m.add_method_mut("set_raw", |_, this, bytes: LuaString| {
            this.set_raw(bytes.as_bytes().as_ref())
        });

        m.add_method("len", |_, this, ()| Ok(this.len() as i64));
        m.add_method("is_empty", |_, this, ()| Ok(this.is_empty()));
        m.add_method("clear", |_, this, ()| Ok(this.clear()));

        m.add_meta_method(LuaMetaMethod::Index, |lua, this, key: LuaValue| {
            let LuaValue::String(s) = key else {
                return Ok(LuaValue::Nil);
            };
            match &*s.to_str()? {
                "text" => {
                    let t = this.get_text()?;
                    Ok(LuaValue::String(lua.create_string(&t)?))
                }
                "raw" => Ok(LuaValue::String(this.get_raw(lua)?)),
                "length" | "len" => Ok(LuaValue::Integer(this.len() as i64)),
                "empty" | "is_empty" => Ok(LuaValue::Boolean(this.is_empty())),
                "get_text" | "set_text" | "get_raw" | "set_raw" | "clear" => {
                    let ud = lua.create_userdata(this.clone())?;
                    let f: LuaFunction = ud.get(s)?;
                    Ok(LuaValue::Function(f))
                }
                _ => Ok(LuaValue::Nil),
            }
        });

        m.add_meta_method_mut(
            LuaMetaMethod::NewIndex,
            |_, this, (key, val): (LuaValue, LuaValue)| {
                let LuaValue::String(s) = key else {
                    return Err(LuaError::external("body property must be a string key"));
                };
                match &*s.to_str()? {
                    "text" => {
                        let LuaValue::String(v) = val else {
                            return Err(LuaError::external("body.text must be a string"));
                        };
                        this.set_text(v.to_string_lossy().as_ref())
                    }
                    "raw" => {
                        let LuaValue::String(v) = val else {
                            return Err(LuaError::external("body.raw must be a string (bytes)"));
                        };
                        this.set_raw(v.as_bytes().as_ref())
                    }
                    "length" | "len" | "empty" | "is_empty" => {
                        Err(LuaError::external("read-only property"))
                    }
                    other => Err(LuaError::external(format!(
                        "unknown body property '{other}'"
                    ))),
                }
            },
        );
        m.add_meta_method(LuaMetaMethod::ToString, |_, this, ()| this.get_text());
    }
}

pub fn register_body(lua: &Lua) -> LuaResult<LuaTable> {
    let tbl = lua.create_table()?;
    let new = lua.create_function(|_, v: Option<LuaString>| {
        let bytes = v
            .map(|s| Bytes::from(s.as_bytes().to_vec()))
            .unwrap_or_default();
        Ok(LuaBody::from_bytes(bytes))
    })?;
    tbl.set(KEY_NEW, new)?;
    lua.globals().set("Body", tbl.clone())?;
    Ok(tbl)
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    fn with_lua<F: FnOnce(&Lua) -> LuaResult<()>>(f: F) {
        let lua = Lua::new();
        register_body(&lua).unwrap();
        f(&lua).expect("lua ok");
    }

    #[test]
    fn b01_new_defaults_empty() {
        with_lua(|lua| {
            lua.load(
                r#"
                local Body = Body
                local b = Body.new()
                assert(b:is_empty() == true)
                assert(b:len() == 0)
                -- default text is empty string
                assert(b:get_text() == "")
                -- tostring uses __tostring -> text
                assert(tostring(b) == "")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn b02_methods_text_roundtrip() {
        with_lua(|lua| {
            lua.load(
                r#"
                local b = Body.new()
                b:set_text("hello")
                assert(b:get_text() == "hello")
                assert(b:is_empty() == false)
                assert(b:len() == 5)
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn b03_methods_raw_roundtrip_with_nul() {
        with_lua(|lua| {
            lua.load(
                r#"
                local b = Body.new()
                -- include NUL and non-ASCII bytes
                local s = "a\0b\255"
                b:set_raw(s)
                local got = b:get_raw()
                assert(got == s)
                -- tostring(b) decodes as UTF-8; non-UTF8 may error in Rust,
                -- but here "\255" may be treated as 0xFF by Lua; ensure raw path works:
                assert(b:len() == #s)
                assert(b:is_empty() == false)
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn b04_property_sugar_text() {
        with_lua(|lua| {
            lua.load(
                r#"
                local b = Body.new()
                b.text = "world"
                assert(b.text == "world")
                -- sugar sets underlying bytes
                assert(b:len() == 5)
                -- updating via methods reflects in property
                b:set_text("x")
                assert(b.text == "x")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn b05_property_sugar_raw_with_bytes() {
        with_lua(|lua| {
            lua.load(
                r#"
                local b = Body.new()
                local payload = "\0\1\2xyz"
                b.raw = payload
                assert(b.raw == payload)
                assert(b:len() == #payload)
                -- mix: set_raw then read text (may be invalid UTF-8; we only assert len stability)
                b.raw = "abc"
                assert(b.text == "abc")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn b06_readonly_len_and_empty() {
        with_lua(|lua| {
            let ok = lua
                .load(
                    r#"
                    local b = Body.new("hi")
                    local ok1, err1 = pcall(function() b.len = 99 end)
                    local ok2, err2 = pcall(function() b.is_empty = false end)
                    return ok1, ok2
                "#,
                )
                .eval::<(bool, bool)>()?;
            assert_eq!(ok, (false, false));
            Ok(())
        });
    }

    #[test]
    fn b07_tostring_reflects_text() {
        with_lua(|lua| {
            lua.load(
                r#"
                local b = Body.new()
                b.text = "hello"
                assert(tostring(b) == "hello")
                b.text = "bye"
                assert(tostring(b) == "bye")
            "#,
            )
            .exec()
        });
    }

    #[test]
    fn b08_constructor_with_initial_bytes() {
        with_lua(|lua| {
            lua.load(
                r#"
                local Body = Body
                local b = Body.new("seed")
                assert(b.text == "seed")
                assert(b:len() == 4)
            "#,
            )
            .exec()
        });
    }
}
