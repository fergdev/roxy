use boa_engine::object::builtins::JsArray;
use boa_engine::value::Convert;
use boa_engine::{
    Context, Finalize, JsData, JsResult, JsString, JsValue, Trace, js_error, js_string,
};
use boa_interop::{JsClass, js_class};
use std::cell::RefCell;
use std::rc::Rc;
use url::form_urlencoded::Serializer;

#[derive(Debug, Clone, JsData, Trace, Finalize)]
#[boa_gc(unsafe_no_drop)]
pub(crate) struct UrlSearchParams {
    #[unsafe_ignore_trace]
    pub(crate) url: Rc<RefCell<url::Url>>,
}

impl UrlSearchParams {
    fn with_url_mut<R>(&self, f: impl FnOnce(&mut url::Url) -> R) -> JsResult<R> {
        let mut u = self.url.borrow_mut();
        Ok(f(&mut u))
    }

    fn read_pairs(&self) -> JsResult<Vec<(String, String)>> {
        self.with_url_mut(|u| {
            u.query_pairs()
                .map(|(k, v)| (k.into_owned(), v.into_owned()))
                .collect()
        })
    }

    fn length(&self) -> usize {
        self.url.borrow().query_pairs().count()
    }

    fn write_pairs(&self, pairs: &[(String, String)]) -> JsResult<()> {
        self.with_url_mut(|u| {
            let mut s = Serializer::new(String::new());
            for (k, v) in pairs {
                s.append_pair(k, v);
            }
            let new_q = s.finish();
            if new_q.is_empty() {
                u.set_query(None);
            } else {
                u.set_query(Some(&new_q));
            }
        })
    }
}

js_class! {
    class UrlSearchParams as "URLSearchParams" {
        property length {
            fn get(this: JsClass<UrlSearchParams>) -> JsResult<usize> {
                Ok(this.borrow().length())
            }
        }
        constructor(query: Option<Convert<String>>) {
            if let Some(Convert(ref q)) = query {
                let mut u = url::Url::parse("http://dummy/")
                    .map_err(|_| js_error!(TypeError: "Invalid query string"))?;
                let clean = q.strip_prefix('?').unwrap_or(q);
                if clean.is_empty() {
                    u.set_query(None);
                } else {
                    u.set_query(Some(clean));
                }
                Ok(Self { url: Rc::new(RefCell::new(u)) })
            } else {
                Err(js_error!(TypeError: "Illegal constructor"))
            }
        }

        fn append(this: JsClass<UrlSearchParams>, key: Convert<String>, value: Convert<String>) -> JsResult<()> {
            this.borrow().with_url_mut(|url| {
                url.query_pairs_mut().append_pair(&key.0, &value.0);
            })
        }

        fn set(this: JsClass<UrlSearchParams>, key: Convert<String>, value: Convert<String>) -> JsResult<()> {
            let pairs = this.borrow().read_pairs()?;
            let k = key.0.to_owned();
            let v = value.0.to_owned();
            let mut pairs = pairs.iter().filter(|(pk, _)| *pk != k).map(|kp| kp.to_owned()).collect::<Vec<_>>();
            pairs.push((k, v));
            this.borrow().write_pairs(&pairs)
        }

        fn get(this: JsClass<UrlSearchParams>, key: Convert<String>) -> JsResult<JsValue> {
            for (k, v) in this.borrow().read_pairs()? {
                if k == key.0 { return Ok(JsValue::from(JsString::from(v))); }
            }
            Ok(JsValue::null())
        }

        fn get_all as "getAll" (this: JsClass<UrlSearchParams>, key: Convert<String>, context: &mut Context) -> JsResult<JsArray> {
            let mut out :Vec<JsValue> = vec![];
            for (k, v) in this.borrow().read_pairs()? {
                if k == key.0 { out.push(JsValue::from(js_string!(v))) }
            }
            Ok(JsArray::from_iter(out, context))
        }

        fn has(this: JsClass<UrlSearchParams>, key: Convert<String>) -> JsResult<bool> {
            Ok(this.borrow().read_pairs()?.iter().any(|(k, _)| *k == key.0))
        }

        fn delete(this: JsClass<UrlSearchParams>, key: Convert<String>) -> JsResult<()> {
            let mut pairs = this.borrow().read_pairs()?;
            pairs.retain(|(k, _)| *k != key.0);
            this.borrow().write_pairs(&pairs)
        }

        fn clear(this: JsClass<UrlSearchParams>) -> JsResult<()> {
            this.borrow().write_pairs(&[])
        }

        fn to_string as "toString"(this: JsClass<UrlSearchParams>) -> JsResult<JsString> {
            let mut s = Serializer::new(String::new());
            for (k, v) in this.borrow().read_pairs()? { s.append_pair(&k, &v); }
            Ok(JsString::from(s.finish()))
        }
    }
}

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::interceptor::js::tests::setup;
    use boa_engine::Source;

    #[test]
    fn urlsearchparams_constructor_from_string_parses_pairs() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const sp = new URLSearchParams("a=1&b=2&a=3");
            assertEqual(sp.get("a"), "1", "get returns first match");
            assertEqual(sp.get("b"), "2", "single key parses");
            assertTrue(sp.has("a") && sp.has("b"), "has() works");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn urlsearchparams_constructor_strips_leading_question_mark() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const sp = new URLSearchParams("?x=10&y=20");
            assertEqual(sp.get("x"), "10", "leading ? accepted");
            assertEqual(sp.get("y"), "20", "both keys parse with ?");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn urlsearchparams_constructor_without_args_throws() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            let threw = false;
            try { new URLSearchParams(); } catch (e) { threw = true; }
            assertTrue(threw, "no-arg constructor must throw");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn urlsearchparams_append_adds_pairs_and_preserves_existing() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const sp = new URLSearchParams("a=1");
            sp.append("a", "2");
            sp.append("b", "x");
            assertEqual(sp.toString(), "a=1&a=2&b=x", "append preserves existing and order");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn urlsearchparams_set_replaces_first_and_dedupes_later() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const sp = new URLSearchParams("a=1&a=2&b=3");
            sp.set("a", "99");
            assertEqual(sp.get("a"), "99", "set replaces first value");
            assertEqual(sp.toString(), "b=3&a=99", "set dedupes subsequent entries");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn urlsearchparams_get_returns_null_when_missing() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const sp = new URLSearchParams("a=1");
            assertEqual(sp.get("b"), null, "missing key returns null");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn urlsearchparams_has_and_delete() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const sp = new URLSearchParams("x=1&y=2&x=3");
            assertTrue(sp.has("x") && sp.has("y"), "has before delete");
            sp.delete("x");
            assertTrue(!sp.has("x"), "delete removes all entries for key");
            assertEqual(sp.toString(), "y=2", "only y remains");
        "#,
        ))
        .unwrap();
    }

    #[test]
    fn urlsearchparams_to_string_roundtrip() {
        let mut ctx = setup();
        ctx.eval(Source::from_bytes(
            r#"
            const sp = new URLSearchParams("p=1&q=hello+world&p=2");
            const s = sp.toString();
            assertEqual(s, "p=1&q=hello+world&p=2", "toString preserves duplicates and encoding");

            // roundtrip: construct a new instance from the string
            const sp2 = new URLSearchParams(s);
            assertEqual(sp2.get("p"), "1", "roundtrip first p");
            assertEqual(sp2.get("q"), "hello world", "roundtrip decoded q");
        "#,
        ))
        .unwrap();
    }
}
