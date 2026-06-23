use std::{cell::RefCell, rc::Rc};

use boa_engine::{
    Context, JsData, JsResult, JsValue, boa_class, js_error,
    object::builtins::{AlignedVec, JsArrayBuffer},
    value::TryFromJs,
};
use boa_gc::{Finalize, Trace};
use bytes::Bytes;

#[derive(Debug, Clone, Trace, Finalize, JsData)]
pub(crate) struct JsBody {
    #[unsafe_ignore_trace]
    pub inner: Rc<RefCell<Bytes>>,
}

impl Default for JsBody {
    fn default() -> Self {
        Self::from_bytes(Bytes::new())
    }
}

#[boa_class(rename = "Body")]
#[boa(rename_all = "camelCase")]
impl JsBody {
    #[boa(constructor)]
    fn new(js_value: JsValue) -> JsResult<Self> {
        if js_value.is_undefined() || js_value.is_null() {
            return Ok(Self::default());
        }
        if let Some(s) = js_value.as_string() {
            return Ok(Self::from_bytes(Bytes::from(
                s.to_std_string_escaped().into_bytes(),
            )));
        }
        if let Some(n) = js_value.as_number() {
            return Ok(Self::from_bytes(Bytes::from(n.to_string().into_bytes())));
        }
        Err(js_error!(TypeError: "Invalid type {value} has no data"))
    }

    #[boa(getter)]
    pub fn length(&self) -> usize {
        self.inner.borrow().len()
    }

    #[boa(rename = "isEmpty")]
    pub fn is_empty(&self) -> bool {
        self.inner.borrow().is_empty()
    }

    #[boa(getter)]
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.inner.borrow()).to_string()
    }

    #[boa(setter)]
    #[boa(method)]
    #[boa(rename = "text")]
    pub fn set_text(&self, js_value: JsValue, context: &mut Context) -> JsResult<()> {
        *self.inner.borrow_mut() =
            Bytes::from(js_value.to_string(context)?.to_std_string_escaped());
        Ok(())
    }

    #[boa(method)]
    pub fn bytes(&self) -> Vec<u8> {
        self.inner.borrow().to_vec()
    }

    #[boa(getter)]
    fn raw(&self, context: &mut Context) -> JsResult<JsValue> {
        let bytes = self.inner.borrow();
        let mut aligned = AlignedVec::<u8>::new(0);
        aligned.extend_from_slice(bytes.as_ref());
        let buf = JsArrayBuffer::from_byte_block(aligned, context)?;
        Ok(buf.into())
    }

    #[boa(setter)]
    #[boa(method)]
    #[boa(rename = "raw")]
    fn set_raw(&self, value: JsValue, context: &mut Context) -> JsResult<()> {
        if let Ok(buf) = JsArrayBuffer::try_from_js(&value, context) {
            let data = buf
                .data()
                .ok_or(js_error!(TypeError: "ArrayBuffer has no data"))?;
            *self.inner.borrow_mut() = Bytes::from(data.to_vec());
        }

        Ok(())
    }

    fn clear(&self) {
        self.inner.borrow_mut().clear();
    }

    #[boa(method)]
    #[boa(rename = "toString")]
    pub fn to_string_js(&self) -> String {
        self.text()
    }
}

impl JsBody {
    pub fn from_bytes(bytes: Bytes) -> Self {
        Self {
            inner: Rc::new(RefCell::new(bytes)),
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

    #[test]
    fn body_clear_removes_body() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const b = new Body("x");
            b.clear();
            assertEqual(b.text, "", "clear removes text");
            assertEqual(b.length, 0, "clear length is 0");
            assertEqual(b.isEmpty(), true, "clear length is 0");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn body_is_empty_true_by_default() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const b = new Body();
            assertEqual(b.isEmpty(), true, "empty by default");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn body_is_empty_false_with_data() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const b = new Body("data");
            assertEqual(b.isEmpty(), false, "false with data");
        "#,
        ))
        .unwrap();
    }
}
