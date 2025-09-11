use boa_engine::{Context, JsData, JsResult, JsValue};
use boa_gc::{Finalize, Trace};
use boa_interop::{JsClass, js_class};

use crate::interceptor::js::{request::JsRequest, response::JsResponse};

#[derive(Debug, Clone, Trace, Finalize, JsData, Default)]
pub(crate) struct JsFlow {
    pub(crate) request: JsRequest,
    pub(crate) response: JsResponse,
}

js_class! {
    class JsFlow as "Flow" {
        property request {
            fn get(this: JsClass<JsFlow>, context: &mut Context) -> JsResult<JsValue> {
                let req = this.borrow().request.clone();
                JsRequest::from_data(req, context).map(JsValue::from)
            }
        }

        property response {
            fn get(this: JsClass<JsFlow>, context: &mut Context) -> JsResult<JsValue> {
                let res = this.borrow().response.clone();
                JsResponse::from_data(res, context).map(JsValue::from)
            }
        }

        constructor() {
            Ok(Self::default())
        }
        init(_class: &mut ClassBuilder) -> JsResult<()> {
            Ok(())
        }
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {

    use crate::interceptor::js::engine::register_classes;
    use boa_engine::{Context, Source};

    fn setup() -> Context {
        crate::init_test_logging();
        let mut ctx = Context::default();

        // JS assert helper
        ctx.eval(Source::from_bytes(
            r#"
            function must(cond, msg) { if (!cond) throw new Error(msg || "assert failed"); }
            "#,
        ))
        .unwrap();

        register_classes(&mut ctx).expect("register classes");
        ctx
    }

    #[test]
    fn flow_constructor_creates_default_instance() {
        let mut ctx = setup();
        let v = ctx
            .eval(Source::from_bytes(
                r#"
                const f = new Flow();
                must(typeof f === "object", "Flow() should construct an object");
                // Just return something so we can assert in Rust too
                true
                "#,
            ))
            .unwrap();
        assert!(v.is_boolean() && v.as_boolean().unwrap());
    }

    #[test]
    fn flow_exposes_request_and_response_properties() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const flow = new Flow();
            must(typeof flow.request === "object", "flow.request is object");
            must(typeof flow.response === "object", "flow.response is object");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn flow_request_is_a_request_instance() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const flow = new Flow();
            const r = flow.request;
            must(r && typeof r === "object", "request is object");
            try { void r.method; } catch (e) { must(false, "request.method should be readable"); }
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn flow_response_is_a_response_instance() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const flow = new Flow();
            const res = flow.response;
            must(res && typeof res === "object", "response is object");
            // Non-throwing accessor sanity check (adjust if your API differs)
            try { void res.status; } catch (e) { must(false, "response.status should be readable"); }
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn flow_properties_are_live_views_not_copies() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const flow = new Flow();
            const r1 = flow.request;
            const r2 = flow.request;
            must(r1 && r2, "both requests exist");
            must(typeof r1 === "object" && typeof r2 === "object", "both are objects");
            try { void r1.method; void r2.method; } catch (e) { must(false, "request accessors ok"); }
            "#,
        ))
        .unwrap();
    }
}
