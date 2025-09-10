use boa_engine::{Context, JsData, JsResult, JsValue, js_error};
use boa_gc::{Finalize, Trace};
use boa_interop::{JsClass, js_class};

use crate::interceptor::js::{request::JsRequest, response::JsResponse};

#[derive(Debug, Clone, Trace, Finalize, JsData)]
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
            Err(js_error!(TypeError: "Illegal constructor"))
        }
        init(_class: &mut ClassBuilder) -> JsResult<()> {
            Ok(())
        }
    }
}
