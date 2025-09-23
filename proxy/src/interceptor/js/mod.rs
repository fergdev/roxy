mod body;
mod constants;
pub mod engine;
mod flow;
mod headers;
mod logger;
mod query;
mod request;
mod response;
mod url;
mod util;

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::interceptor::js::engine::register_classes;
    use boa_engine::{Context, Source};

    pub(crate) fn setup() -> Context {
        crate::init_test_logging();
        let mut ctx = Context::default();
        register_classes(&mut ctx).expect("register classes");
        ctx.eval(Source::from_bytes(
            r#"
            function assertEqual(a, b, msg) {
                if (a !== b) {
                    msg = msg || `assertEqual failed: ${a} !== ${b}`
                    console.log(msg);
                    throw new Error(msg);
                }
            }
            function assertTrue(cond, msg) {
                if (!cond) {
                    msg = msg || `assertTrue failed`
                    console.log(msg);
                    throw new Error(msg);
                }
            }
            function assertFalse(cond, msg) {
                if (cond) {
                    msg = msg || `assertFalse failed`
                    console.log(msg);
                    throw new Error(msg);
                }
            }
            function assertNull(cond, msg) {
                if (cond !== null) {
                    msg = msg || `assertNull failed`
                    console.log(msg);
                    throw new Error(msg);
                }
            }
            "#,
        ))
        .unwrap();
        ctx
    }
}
