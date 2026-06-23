use std::{cell::RefCell, rc::Rc, str::FromStr};

use boa_engine::{
    Context, JsObject, JsResult, JsValue, boa_class, class::Class, js_error, js_string,
};
use roxy_shared::version::HttpVersion;
use tracing::info;

use crate::{
    flow::InterceptedRequest,
    interceptor::js::{
        body::JsBody,
        headers::{HeaderList, JsHeaders},
        url::JsUrl,
    },
};

#[derive(Debug, Clone, boa_engine::Trace, boa_engine::Finalize, boa_engine::JsData)]
#[boa_gc(unsafe_no_drop)]
pub(crate) struct JsRequest {
    #[unsafe_ignore_trace]
    pub(crate) req: Rc<RefCell<InterceptedRequest>>,
    #[unsafe_ignore_trace]
    pub(crate) body: JsBody,
    #[unsafe_ignore_trace]
    pub(crate) url_obj: Rc<RefCell<Option<JsObject>>>,
    #[unsafe_ignore_trace]
    pub(crate) headers: HeaderList,
    #[unsafe_ignore_trace]
    pub(crate) trailers: HeaderList,
}

impl Default for JsRequest {
    fn default() -> Self {
        Self {
            req: Rc::new(RefCell::new(InterceptedRequest::default())),
            body: JsBody::default(),
            url_obj: Rc::new(RefCell::new(None)),
            headers: HeaderList::default(),
            trailers: HeaderList::default(),
        }
    }
}

#[boa_class(rename = "Request")]
impl JsRequest {
    fn ensure_url(&self, ctx: &mut Context) -> JsResult<JsObject> {
        info!("ensuring URL object for request");
        if let Some(o) = self.url_obj.borrow().clone() {
            info!("yep {o:?}");
            return Ok(o);
        }
        let req = self.req.borrow();
        let o = make_url_for_request(ctx, &req)?;
        info!("no url defined yet, made new one: {o:?}");
        *self.url_obj.borrow_mut() = Some(o.clone());
        Ok(o)
    }

    #[boa(constructor)]
    fn new() -> Self {
        Self::default()
    }

    #[boa(getter)]
    fn method(&self) -> String {
        self.req.borrow().method.to_string()
    }

    #[boa(setter)]
    #[boa(rename = "method")]
    fn set(&self, value: String) -> JsResult<()> {
        let m = http::Method::from_str(&value)
            .map_err(|e| js_error!(TypeError: "Invalid method: {}", e))?;
        self.req.borrow_mut().method = m;
        Ok(())
    }

    #[boa(getter)]
    fn version(&self) -> String {
        self.req.borrow().version.to_string()
    }

    #[boa(setter)]
    #[boa(rename = "version")]
    fn set_version(&self, value: String) -> JsResult<()> {
        let version: HttpVersion = value
            .parse()
            .map_err(|_| js_error!(TypeError: "Invalid HTTP version"))?;
        self.req.borrow_mut().version = version;
        Ok(())
    }

    #[boa(getter)]
    fn headers(&self, context: &mut Context) -> JsResult<JsValue> {
        let proto = crate::interceptor::js::util::class_proto(context, JsHeaders::NAME)?;
        let h = JsHeaders {
            headers: self.headers.clone(),
        };
        let obj = JsObject::from_proto_and_data(proto, h);
        Ok(JsValue::new(obj))
    }

    #[boa(getter)]
    fn trailers(&self, context: &mut Context) -> JsResult<JsValue> {
        let proto = crate::interceptor::js::util::class_proto(context, JsHeaders::NAME)?;
        let h = JsHeaders {
            headers: self.trailers.clone(),
        };
        let obj = JsObject::from_proto_and_data(proto, h);
        Ok(JsValue::new(obj))
    }

    #[boa(getter)]
    fn body(&self, context: &mut Context) -> JsResult<JsValue> {
        let proto = crate::interceptor::js::util::class_proto(context, JsBody::NAME)?;
        let h = self.body.clone();
        let obj = JsObject::from_proto_and_data(proto, h);
        Ok(JsValue::new(obj))
    }

    #[boa(getter)]
    fn url(&self, context: &mut Context) -> JsResult<JsValue> {
        let url_obj = self.ensure_url(context)?;
        Ok(JsValue::new(url_obj))
    }

    #[boa(setter)]
    #[boa(rename = "url")]
    fn set_url(&self, value: JsValue, context: &mut Context) -> JsResult<()> {
        if let Some(o) = value.as_object() {
            *self.url_obj.borrow_mut() = Some(o.clone());
            return Ok(());
        }

        if value.is_string() {
            let href = value.to_string(context)?.to_std_string_escaped();
            let url_obj = {
                let base = "http://localhost";
                let url_ctor = context
                    .global_object()
                    .get(js_string!(JsUrl::NAME), context)?;
                url_ctor
                    .as_object()
                    .ok_or_else(|| js_error!("URL constructor missing"))?
                    .construct(
                        &[
                            JsValue::new(js_string!(href)),
                            JsValue::new(js_string!(base)),
                        ],
                        None,
                        context,
                    )?
            };
            *self.url_obj.borrow_mut() = Some(url_obj);
            return Ok(());
        }

        Err(js_error!(TypeError: "Request.url must be a URL or string"))
    }
}

fn make_url_for_request2(ctx: &mut Context) -> JsResult<JsObject> {
    let base = "http://localhost";

    let url_ctor = ctx.global_object().get(js_string!(JsUrl::NAME), ctx)?;
    let url_obj = url_ctor
        .as_object()
        .ok_or_else(|| js_error!("URL constructor missing"))?
        .construct(&[js_string!(base).into()], None, ctx)?;
    Ok(url_obj)
}

fn make_url_for_request(ctx: &mut Context, req: &InterceptedRequest) -> JsResult<JsObject> {
    let href = req.uri.to_string();
    info!("href for request URL: {href}");
    let base = "http://localhost";

    let url_ctor = ctx.global_object().get(js_string!(JsUrl::NAME), ctx)?;
    let url_obj = url_ctor
        .as_object()
        .ok_or_else(|| js_error!("URL constructor missing"))?
        .construct(
            &[
                JsValue::new(js_string!(href)),
                JsValue::new(js_string!(base)),
            ],
            None,
            ctx,
        )?;

    Ok(url_obj)
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::interceptor::js::tests::setup;
    use boa_engine::Source;

    #[test]
    fn request_constructor_default_succeeds() {
        let mut ctx = setup();
        let ok = ctx
            .eval(Source::from_bytes(
                r#"
                const r = new Request();
                assertTrue(typeof r === "object", "Request should construct an object");
                // default method and version are readable (format, not asserting exact)
                assertTrue(typeof r.method === "string", "method is string");
                assertTrue(typeof r.version === "string", "version is string");
                true
            "#,
            ))
            .unwrap();
        assert!(ok.is_boolean() && ok.as_boolean().unwrap());
    }

    #[test]
    fn request_method_set_get_roundtrip() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const r = new Request();
            r.method = "POST";
            assertEqual(r.method, "POST", "method roundtrip");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn request_method_invalid_type_throws() {
        let mut ctx = setup();
        let res = ctx
            .eval(Source::from_bytes(
                r#"
            try {
              const r = new Request();
              r.method = 123; // not a string
              assertTrue(false, "expected TypeError");
            } catch (e) {
              assertTrue(e instanceof TypeError, "TypeError on non-string method");
              true
            }
            "#,
            ))
            .unwrap();
        assert!(res.is_boolean() && res.as_boolean().unwrap());
    }

    #[test]
    fn request_method_invalid_value_throws() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            try {
              const r = new Request();
              r.method = " NOT_A_METHOD ";
              assertTrue(false, "expected TypeError for invalid method");
            } catch (e) {
              assertTrue(e instanceof TypeError, "TypeError for invalid method");
            }
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn request_version_set_get_roundtrip() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const r = new Request();
            r.version = "HTTP/2.0";
            assertEqual(r.version, "HTTP/2.0", "version roundtrip to string format");
            r.version = "HTTP/1.1";
            assertEqual(r.version, "HTTP/1.1", "version downgraded ok");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn request_version_rejects_bad_values() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            try {
              const r = new Request();
              r.version = "HTTP/9.9";
              assertTrue(false, "expected TypeError for unsupported version");
            } catch (e) {
              assertTrue(e instanceof TypeError, "TypeError for bad version");
            }
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn request_headers_returns_headers_object() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const r = new Request();
            const h = r.headers;
            assertTrue(h && typeof h === "object", "headers is object");
            h.set("X-Test", "1");
            assertEqual(h.get("X-Test"), "1", "headers.get after set");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn request_trailers_returns_headers_object() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const r = new Request();
            const t = r.trailers;
            assertTrue(t && typeof t === "object", "trailers is object");
            t.append("X-Trailer", "A");
            t.append("X-Trailer", "B");
            const all = t.getAll("X-Trailer");
            assertTrue(Array.isArray(all) && all.length === 2, "two trailer values");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn request_body_returns_body_object_and_roundtrips_text() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const r = new Request();
            const b = r.body;
            assertTrue(b && typeof b === "object", "body is object");
            b.text = "hello";
            assertEqual(b.text, "hello", "body text roundtrip");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn request_url_getter_returns_url_object() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const r = new Request();
            const u = r.url;
            assertTrue(u && typeof u === "object", "url is object");
            // Not asserting exact fields, just that it behaves like URL (has href/toString)
            assertTrue(typeof u.href === "string", "url.href is string");
            assertTrue(typeof u.toString === "function", "url.toString exists");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn request_url_set_accepts_string() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const r = new Request();
            r.url = "http://example.com/path?x=1";
            const u1 = r.url;
            assertTrue(u1 && typeof u1 === "object", "url object after string set");
            assertTrue(typeof u1.href === "string", "url.href exists");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn request_url_set_accepts_url_object() {
        let mut ctx = setup();

        ctx.eval(Source::from_bytes(
            r#"
                const u = new URL("http://localhost/base", "http://localhost");
                const r = new Request();
                r.url = u;
                const got = r.url;
                assertTrue(got && typeof got === "object", "url is object after object set");
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn request_url_set_invalid_type_throws() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            try {
              const r = new Request();
              r.url = 42; // not a URL object and not a string
              assertTrue(false, "expected TypeError");
            } catch (e) {
              assertTrue(e instanceof TypeError, "TypeError for invalid url assignment");
            }
            "#,
        ))
        .unwrap();
    }

    #[test]
    fn request_properties_live_views_not_copies() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const r = new Request();
            const h1 = r.headers;
            const h2 = r.headers;
            h1.set("X-Live", "yes");
            assertEqual(h2.get("X-Live"), "yes", "same live headers view");

            const t1 = r.trailers;
            const t2 = r.trailers;
            t1.append("X-Trail", "A");
            assertEqual(t2.has("X-Trail"), true, "same live trailers view");

            const b1 = r.body;
            const b2 = r.body;
            b1.text = "ok";
            assertEqual(b2.text, "ok", "same live body view");
            "#,
        ))
        .unwrap();
    }
}
