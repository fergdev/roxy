use std::{cell::RefCell, ops::Deref, rc::Rc};

use boa_engine::{Context, JsObject, JsResult, JsString, JsValue, js_error, js_string};
use boa_interop::{JsClass, js_class};
use http::StatusCode;
use roxy_shared::version::HttpVersion;

use crate::flow::InterceptedResponse;
use crate::interceptor::js::body::JsBody;
use crate::interceptor::js::headers::{HeaderList, JsHeaders};
use crate::interceptor::js::util::class_proto;

#[derive(Debug, Clone, boa_engine::Trace, boa_engine::Finalize, boa_engine::JsData)]
#[boa_gc(unsafe_no_drop)]
pub(crate) struct JsResponse {
    #[unsafe_ignore_trace]
    pub(crate) resp: Rc<RefCell<Option<InterceptedResponse>>>,
    #[unsafe_ignore_trace]
    pub(crate) body: JsBody,
    #[unsafe_ignore_trace]
    pub(crate) headers: HeaderList,
    #[unsafe_ignore_trace]
    pub(crate) trailers: HeaderList,
}

impl JsResponse {
    pub fn into_intercepted(self) -> Option<InterceptedResponse> {
        let mut resp = self.resp.borrow().clone().unwrap_or_default();

        let body_bytes = self.body.inner.borrow();
        if !body_bytes.is_empty() {
            resp.body = body_bytes.clone();
        }

        let headers = self.headers.borrow();
        if !headers.is_empty() {
            resp.headers = headers.clone();
        }

        let trailers = self.trailers.borrow();
        if !trailers.is_empty() {
            resp.trailers = Some(trailers.clone());
        }

        if resp.status != 0
            || !resp.body.is_empty()
            || !resp.headers.is_empty()
            || resp.trailers.is_some()
        {
            Some(resp)
        } else {
            None
        }
    }
}

js_class! {
    class JsResponse as "Response" {
        property version {
            fn get(this: JsClass<JsResponse>) -> JsString {
                let version = if let Some(res) = this.borrow().resp.borrow().deref() {
                    res.version.to_string()
                } else {
                    String::new()
                };
                js_string!(version)
            }

            fn set(this: JsClass<JsResponse>, value: JsValue, context: &mut Context) -> JsResult<()> {
                if value.is_string() {
                    let version : HttpVersion = value.to_string(context)?.to_std_string_escaped().parse()
                        .map_err(|_| js_error!(TypeError: "Invalid HTTP version"))?;
                    let this = this.borrow();
                    let mut opt = this.resp.borrow_mut();
                    let resp = opt.get_or_insert_with(InterceptedResponse::default);
                    resp.version = version;
                    return Ok(());
                }
                Err(js_error!(TypeError: "Request.method must be a string"))
            }
        }
        property headers {
            fn get(this: JsClass<JsResponse>, context: &mut Context) -> JsResult<JsValue> {
                let list = this.borrow().headers.clone();
                JsHeaders::from_data(JsHeaders { headers: list }, context).map(JsValue::from)
            }
        }

        property trailers {
            fn get(this: JsClass<JsResponse>, context: &mut Context) -> JsResult<JsValue> {
                let list = this.borrow().trailers.clone();
                JsHeaders::from_data(JsHeaders { headers: list }, context).map(JsValue::from)
            }
        }

        property status {
            fn get(this: JsClass<JsResponse>) -> JsResult<JsValue> {
                let status = if let Some(res) = this.borrow().resp.borrow().deref() {
                    res.status.as_u16() as i32
                } else {
                    0
                };
                Ok(JsValue::Integer(status))
            }

            fn set(this: JsClass<JsResponse>, value: JsValue, context: &mut Context) -> JsResult<()> {
                let this = this.borrow();
                let mut opt = this.resp.borrow_mut();
                let resp = opt.get_or_insert_with(InterceptedResponse::default);

                let code = if value.is_integer() || value.is_number() {
                    value.to_i32(context)?
                } else {
                    return Err(js_error!(TypeError: "status must be a number"));
                };

                resp.status = StatusCode::from_u16(code as u16)
                    .map_err(|_| js_error!(TypeError: "invalid HTTP status code"))?;
                Ok(())
            }
        }

        property body {
            fn get(this: JsClass<JsResponse>, context: &mut Context) -> JsResult<JsValue> {
                let proto = class_proto(context, JsBody::NAME)?;
                let h = this.borrow().body.clone();
                let obj = JsObject::from_proto_and_data(proto, h);
                Ok(JsValue::Object(obj))
            }
        }
        constructor() {
            Err(js_error!(TypeError: "Illegal constructor"))
        }

        init(_class: &mut ClassBuilder) -> JsResult<()> {
            Ok(())
        }
    }
}
