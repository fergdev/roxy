use mlua::prelude::*;

pub(crate) const KEY_NEW: &str = "new";

pub(crate) fn lua_val_to_str(val: LuaValue) -> LuaResult<String> {
    Ok(match val {
        LuaValue::String(s) => s.to_str()?.to_string(),
        LuaValue::Integer(i) => i.to_string(),
        LuaValue::Number(n) => {
            if n.fract() == 0.0 {
                (n as i64).to_string()
            } else {
                n.to_string()
            }
        }
        LuaValue::Boolean(b) => {
            if b {
                "true".into()
            } else {
                "false".into()
            }
        }
        other => {
            return Err(LuaError::external(format!(
                "unsupported header value type: {other:?}"
            )));
        }
    })
}
