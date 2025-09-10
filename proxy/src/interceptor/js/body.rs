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
    pub(crate) fn register(context: &mut Context) -> JsResult<()> {
        context.register_global_class::<Self>()?;
        Ok(())
    }
    pub(crate) fn new(data: Bytes) -> Self {
        Self {
            inner: Rc::new(RefCell::new(data)),
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
        constructor(value: String) {
            Ok(JsBody::new(Bytes::from(value)))
        }

        init(_class: &mut ClassBuilder) -> JsResult<()> {
            Ok(())
        }
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use super::*;
    use boa_engine::{Context, Source};

    fn setup() -> Context {
        let mut ctx = Context::default();
        JsBody::register(&mut ctx).expect("register Body");
        ctx
    }

    #[test]
    fn body_js_happy_paths() {
        let mut ctx = setup();

        ctx.eval(
            Source::from_bytes(
            r#"
            function must(cond, msg) {
                if (!cond) throw new Error(msg || "assert failed");
            }

            // new Body("seed")
            const b1 = new Body("seed");
            must(b1.text === "seed", "text roundtrip");
            // overwrite via text
            b1.text = "hello";
            must(b1.text === "hello", "text setter/getter");

            // set raw using ArrayBuffer and verify roundtrip
            const bytes = new Uint8Array([0x61, 0x00, 0x62, 0xFF]); // "a\0b\xff"
            b1.raw = bytes.buffer;

            const got = new Uint8Array(b1.raw);
            must(got.length === 4, "raw length");
            must(got[0] === 0x61 && got[1] === 0x00 && got[2] === 0x62 && got[3] === 0xFF, "raw content");

            // after setting raw, text should reflect UTF-8 decoding (lossy is fine);
            // we only check that reading doesn't throw and returns a string.
            must(typeof b1.text === "string", "text after raw set is string");

            // new Body() default works via your constructor (currently requires a String).
            // Since your constructor signature is `constructor(value: String)`,
            // constructing without args isn't allowed here; we just ensure string path works.
            const b2 = new Body("");
            must(b2.text === "", "empty constructor string ok");
        "#)).unwrap();
    }
}
