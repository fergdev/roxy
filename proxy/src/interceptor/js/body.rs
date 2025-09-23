use std::{cell::RefCell, rc::Rc};

use boa_engine::{
    Context, JsData, JsResult, JsString, JsValue, js_error, js_string,
    object::builtins::JsArrayBuffer, value::TryFromJs,
};
use boa_gc::{Finalize, Trace};
use boa_interop::{JsClass, js_class};
use bytes::Bytes;

#[derive(Debug, Clone, Trace, Finalize, JsData)]
pub(crate) struct JsBody {
    #[unsafe_ignore_trace]
    pub inner: Rc<RefCell<Bytes>>,
}

impl Default for JsBody {
    fn default() -> Self {
        Self::new(Bytes::new())
    }
}

impl JsBody {
    pub(crate) fn new(data: Bytes) -> Self {
        Self {
            inner: Rc::new(RefCell::new(data)),
        }
    }
    fn new_value(value: &JsValue) -> JsResult<Self> {
        if value.is_undefined() || value.is_null() {
            return Ok(Self::new(Bytes::new()));
        }

        match value {
            JsValue::Object(o) => {
                let buf = JsArrayBuffer::from_object(o.clone())?;
                return Ok(Self::new(Bytes::from(
                    buf.data()
                        .ok_or(js_error!(TypeError: "ArrayBuffer has no data"))?
                        .to_owned(),
                )));
            }
            JsValue::Integer(integer) => {
                Ok(Self::new(Bytes::from(integer.to_string().into_bytes())))
            }
            JsValue::String(string) => Ok(Self::new(Bytes::from(
                string.to_std_string_escaped().into_bytes(),
            ))),
            _ => Err(js_error!(TypeError: "Invalid type {value} has no data")),
        }
    }

    fn length(&self) -> JsValue {
        let len = self.inner.borrow().len();
        JsValue::Integer(len as i32)
    }

    fn is_empty(&self) -> JsValue {
        JsValue::Boolean(self.inner.borrow().is_empty())
    }
}

js_class! {
    class JsBody as "Body" {
        property text {
            fn get(this: JsClass<JsBody>) -> JsString {
                let this = this.borrow();
                let bytes = this.inner.borrow();
                let s = String::from_utf8_lossy(&bytes).to_string();
                js_string!(s)
            }

            fn set(this: JsClass<JsBody>, value: JsValue, context: &mut Context) -> JsResult<()> {
                let s = value.to_string(context)?.to_std_string_escaped();
                *this.borrow().inner.borrow_mut() = Bytes::from(s.into_bytes());
                Ok(())
            }
        }

        property raw {
            fn get(this: JsClass<JsBody>, context: &mut Context) -> JsResult<JsValue> {
                let this = this.borrow();
                let bytes = this.inner.borrow();
                let buf = JsArrayBuffer::from_byte_block(bytes.to_vec(), context)?;
                Ok(buf.into())
            }

            fn set(this: JsClass<JsBody>, value: JsValue, context: &mut Context) -> JsResult<()> {
                if let Ok(buf) = JsArrayBuffer::try_from_js(&value, context){
                    let data = buf.data().ok_or(js_error!(TypeError: "ArrayBuffer has no data"))?;
                    *this.borrow().inner.borrow_mut() = Bytes::from(data.to_vec());
                }

                Ok(())
            }
        }
        property length {
            fn get(this: JsClass<JsBody>) -> JsResult<JsValue> {
                let this = this.borrow();
                Ok(this.length())
            }
        }
        property is_empty as "isEmpty" {
            fn get(this: JsClass<JsBody>) -> JsResult<JsValue> {
                let this = this.borrow();
                Ok(this.is_empty())
            }
        }


        constructor(value: JsValue) {
            JsBody::new_value(&value)
        }

        init(_class: &mut ClassBuilder) -> JsResult<()> {
            Ok(())
        }

        fn clear(this: JsClass<JsBody>) -> JsResult<()> {
            *this.borrow().inner.borrow_mut() = Bytes::new();
            Ok(())
        }
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::interceptor::js::tests::setup;
    use boa_engine::Source;

    #[test]
    fn body_constructor_allows_string() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const b = new Body("seed");
            assertEqual(b.text, "seed", "ctor string -> text");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn body_constructor_allows_number() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const b = new Body(123);
            assertEqual(b.text, "123", "ctor number coerces to string");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn body_constructor_allows_empty() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const b = new Body();
            assertEqual(b.text, "", "Empty body should return empty string");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn body_text_get_set_and_coercion() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const b = new Body("x");
            assertEqual(b.text, "x", "initial");

            b.text = "hello";
            assertEqual(b.text, "hello", "text roundtrip");

            b.text = 42;
            assertEqual(b.text, "42", "number coerces");

            b.text = true;
            assertEqual(b.text, "true", "boolean coerces");

            b.text = {};
            assertEqual(b.text, "[object Object]", "object coerces");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn body_raw_roundtrip_arraybuffer() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const b = new Body("");
            const bytes = new Uint8Array([0x61, 0x00, 0x62, 0xFF]);
            b.raw = bytes.buffer;

            const got = new Uint8Array(b.raw);
            assertEqual(got.length, 4, "length");
            assertEqual(got[0], 0x61); 
            assertEqual(got[1], 0x00);
            assertEqual(got[2], 0x62);
            assertEqual(got[3], 0xFF);
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn body_raw_set_wrong_type_is_noop() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const b = new Body("abc");
            const before = b.text;
            b.raw = 123; // not an ArrayBuffer -> no change
            assertEqual(b.text, before, "raw set with non-ArrayBuffer is no-op");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn body_text_after_invalid_utf8_is_still_string() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const b = new Body("");
            const bad = new Uint8Array([0xFF, 0xFF]); // invalid UTF-8
            b.raw = bad.buffer;
            assertTrue(typeof b.text === "string", "text readable even after invalid utf8");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn body_instances_are_independent() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const a = new Body("A");
            const b = new Body("B");
            a.text = "AA";
            assertEqual(a.text, "AA", "a updated");
            assertEqual(b.text, "B", "b unchanged");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn body_raw_returns_arraybuffer() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const b = new Body("hi");
            const ab = b.raw;
            assertTrue(ab instanceof ArrayBuffer, "raw is ArrayBuffer");
            const view = new Uint8Array(ab);
            assertEqual(view.length, 2);
            assertEqual(view[0], 0x68);
            assertEqual(view[1], 0x69);
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn body_text_is_string_after_raw_mutations() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const b = new Body("x");
            const arr = new Uint8Array([0x61,0x62,0x63]); // "abc"
            b.raw = arr.buffer;
            assertEqual(b.text, "abc", "text reflects raw utf-8");
        "#,
        ))
        .unwrap();
    }
}
