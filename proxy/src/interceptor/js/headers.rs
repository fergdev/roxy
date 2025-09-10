use std::{cell::RefCell, rc::Rc};

use boa_engine::{
    Context, JsData, JsError, JsResult, JsValue, js_error, js_string, object::builtins::JsArray,
    value::Convert,
};
use boa_gc::{Finalize, Trace};
use boa_interop::{JsClass, js_class};
use http::{HeaderMap, HeaderName, HeaderValue};

fn to_header_name(name: &str) -> JsResult<HeaderName> {
    HeaderName::from_bytes(name.as_bytes())
        .map_err(|e| js_error!(TypeError: "Invalid header name: {}", e))
}

fn to_header_value(val: &str) -> JsResult<HeaderValue> {
    HeaderValue::from_str(val).map_err(|e| js_error!(TypeError: "Invalid header value: {}", e))
}
pub(crate) type HeaderList = Rc<RefCell<HeaderMap>>;

#[derive(Debug, Trace, Finalize, JsData, Clone)]
#[boa_gc(unsafe_no_drop)]
pub(crate) struct JsHeaders {
    #[unsafe_ignore_trace]
    pub headers: HeaderList,
}

js_class! {
    class JsHeaders as "Headers" {
        constructor() {
            Err(js_error!(TypeError: "Illegal constructor"))
        }

        init(_class: &mut ClassBuilder) -> JsResult<()> {
            Ok(())
        }

        fn get(this: JsClass<JsHeaders>, name: Convert<String>) -> JsResult<JsValue> {
            let name = name.0.clone();
            let found = this.borrow().headers
                .borrow()
                .get(&name)
                .and_then(|v| v.to_str().ok().map(ToString::to_string));

            Ok(match found {
                Some(v) => JsValue::String(js_string!(v)),
                None => JsValue::Null,
            })
        }

        fn get_all(this: JsClass<JsHeaders>, name: Convert<String>, context: &mut Context) -> JsResult<JsValue> {
            let name = to_header_name(&name.0)?;
            let arr = JsArray::new(context);
            for (_, v) in this.borrow().headers.borrow().iter().filter(|(k, _)| name.eq(k)) {
                if let Ok(v) = v.to_str() {
                    arr.push(JsValue::String(js_string!(v)), context)?;
                }
            }
            Ok(arr.into())
        }

        fn set(this: JsClass<JsHeaders>, name: Convert<String>, value: Convert<String>) -> JsResult<()> {
            let name = to_header_name(&name.0)?;
            let value = to_header_value(&value.0)?;

            let this = this.borrow();
            let mut list = this.headers.borrow_mut();
            list.insert(name, value);
            Ok(())
        }

        fn append(this: JsClass<JsHeaders>, name: Convert<String>, value: Convert<String>) -> JsResult<()> {
            let name = to_header_name(&name.0)?;
            let value = to_header_value(&value.0)?;

            this.borrow().headers.borrow_mut().append(name, value);
            Ok(())
        }

        fn delete(this: JsClass<JsHeaders>, name: Convert<String>) -> JsResult<()> {
            let name = to_header_name(&name.0)?;
            let this = this.borrow();
            let mut list = this.headers.borrow_mut();
            list.remove(name);
            Ok(())
        }

        fn has(this: JsClass<JsHeaders>, name: Convert<String>) -> JsResult<bool> {
            let name : HeaderName = name.0.clone().parse().map_err(|e| js_error!(TypeError: "Invalid header name: {}", e))?;
            let has = this.borrow().headers
                .borrow()
                .iter()
                .any(|(k, _)| k.eq(&name));
            Ok(has)
        }
    }
}
