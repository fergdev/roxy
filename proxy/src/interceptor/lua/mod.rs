mod body;
mod constants;
pub mod engine;
mod flow;
mod headers;
mod query;
mod request;
mod response;
mod url;
mod util;

#[allow(clippy::expect_used)]
#[cfg(test)]
mod tests {
    use crate::{init_test_logging, interceptor::lua::engine::register_functions};

    use mlua::prelude::*;

    pub(crate) fn with_lua<F: FnOnce(&Lua) -> LuaResult<()>>(f: F) {
        init_test_logging();
        let lua = Lua::new();
        register_functions(&lua, None).expect("register functions");
        f(&lua).expect("lua ok");
    }
}
