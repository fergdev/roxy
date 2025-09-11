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
        constructor(value: JsValue) {
            JsBody::new_value(&value)
        }

        init(_class: &mut ClassBuilder) -> JsResult<()> {
            Ok(())
        }
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::{init_test_logging, interceptor::js::engine::register_classes};

    use boa_engine::{Context, Source};

    fn setup() -> Context {
        init_test_logging();
        let mut ctx = Context::default();
        ctx.eval(Source::from_bytes(
            r#"
            function must(c,m){ if(!c) throw new Error(m||"assert"); }
        "#,
        ))
        .unwrap();
        register_classes(&mut ctx).expect("register Body");
        ctx
    }

    #[test]
    fn body_constructor_allows_string() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const b = new Body("seed");
            must(b.text === "seed", "ctor string -> text");
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
            must(b.text === "123", "ctor number coerces to string");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn body_constructor_allows_empty() {
        let mut ctx = setup();
        let res = ctx.eval(Source::from_bytes(
            r#"
            const b = new Body();
            must(b.text === "", "Empty body should return empty string");
        "#,
        ));
        assert!(
            res.is_ok(),
            "expected constructor without arg return empty string"
        );
    }

    #[test]
    fn body_text_get_set_and_coercion() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const b = new Body("x");
            must(b.text === "x", "initial");

            b.text = "hello";
            must(b.text === "hello", "text roundtrip");

            // Non-string should coerce via ToString
            b.text = 42;
            must(b.text === "42", "number coerces");

            b.text = true;
            must(b.text === "true", "boolean coerces");

            // Objects: ToString is called (default "[object Object]")
            b.text = {};
            must(b.text === "[object Object]", "object coerces");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn body_raw_roundtrip_arraybuffer() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(r#"
            const b = new Body("");
            const bytes = new Uint8Array([0x61, 0x00, 0x62, 0xFF]); // a \0 b 0xFF
            b.raw = bytes.buffer;

            const got = new Uint8Array(b.raw);
            must(got.length === 4, "length");
            must(got[0] === 0x61 && got[1] === 0x00 && got[2] === 0x62 && got[3] === 0xFF, "content");
        "#)).unwrap();
    }

    #[test]
    fn body_raw_set_wrong_type_is_noop() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const b = new Body("abc");
            const before = b.text;
            b.raw = 123; // not an ArrayBuffer -> no change
            must(b.text === before, "raw set with non-ArrayBuffer is no-op");
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
            must(typeof b.text === "string", "text readable even after invalid utf8");
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
            must(a.text === "AA", "a updated");
            must(b.text === "B", "b unchanged");
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
            must(ab instanceof ArrayBuffer, "raw is ArrayBuffer");
            const view = new Uint8Array(ab);
            must(view.length === 2 && view[0] === 0x68 && view[1] === 0x69, "raw content 'hi'");
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
            must(b.text === "abc", "text reflects raw utf-8");
        "#,
        ))
        .unwrap();
    }
}
