use boa_engine::object::builtins::JsArray;
use boa_engine::{
    Context, Finalize, JsData, JsResult, JsString, JsValue, Trace, boa_class, js_error,
};
use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;
use tracing::info;
use url::form_urlencoded::Serializer;

#[derive(Debug, Clone, JsData, Trace, Finalize)]
#[boa_gc(unsafe_no_drop)]
pub(crate) struct UrlSearchParams {
    #[unsafe_ignore_trace]
    pub(crate) url: Rc<RefCell<url::Url>>,
}

#[boa_class(rename = "URLSearchParams")]
#[boa(rename_all = "camelCase")]
impl UrlSearchParams {
    #[boa(constructor)]
    fn new(query: JsValue) -> JsResult<Self> {
        if let Some(query) = query.as_string() {
            let query = query.to_std_string_escaped();
            let mut u = url::Url::parse("http://dummy/")
                .map_err(|_| js_error!(TypeError: "Invalid query string"))?;
            let clean = query.strip_prefix('?').unwrap_or(&query);
            if clean.is_empty() {
                u.set_query(None);
            } else {
                u.set_query(Some(clean));
            }
            Ok(Self {
                url: Rc::new(RefCell::new(u)),
            })
        } else {
            Err(js_error!(TypeError: "Illegal constructor"))
        }
    }

    #[boa(getter)]
    pub fn length(&self) -> usize {
        self.url.borrow().query_pairs().count()
    }

    fn append(&self, key: String, value: String) -> JsResult<()> {
        info!("appending pair {}={}", key, value);
        with_url_mut(self, |url| {
            url.query_pairs_mut().append_pair(&key, &value);
        })
    }

    fn set(&self, key: String, value: String) -> JsResult<()> {
        let pairs = read_pairs(self)?;
        let mut pairs = pairs
            .iter()
            .filter(|(pk, _)| *pk != key)
            .map(|kp| kp.to_owned())
            .collect::<Vec<_>>();
        pairs.push((key, value));
        write_pairs(self, &pairs)
    }

    fn get(&self, key: String) -> JsResult<JsValue> {
        info!("get pair {}", key);
        for (k, v) in read_pairs(self)? {
            if k == key {
                info!("yep {v}");
                return Ok(JsValue::from(JsString::from(v)));
            }
        }
        info!("nope");
        Ok(JsValue::null())
    }

    #[boa(rename = "getAll")]
    fn get_all(&self, key: String, context: &mut Context) -> JsResult<JsArray> {
        let mut out: Vec<JsValue> = vec![];
        for (k, v) in read_pairs(self)? {
            if k == key {
                out.push(JsString::from(v).into());
            }
        }
        Ok(JsArray::from_iter(out, context))
    }

    fn has(&self, key: String) -> JsResult<bool> {
        Ok(read_pairs(self)?.iter().any(|(k, _)| *k == key))
    }

    fn delete(&self, key: String) -> JsResult<()> {
        let mut pairs = read_pairs(self)?;
        pairs.retain(|(k, _)| *k != key);
        write_pairs(self, &pairs)
    }

    fn clear(&self) -> JsResult<()> {
        write_pairs(self, &[])
    }

    #[boa(rename = "toString")]
    fn to_string(&self) -> JsResult<JsString> {
        let mut s = Serializer::new(String::new());
        for (k, v) in read_pairs(self)? {
            s.append_pair(&k, &v);
        }
        Ok(JsString::from(s.finish()))
    }
}

impl Display for UrlSearchParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "URLSearchParams({})",
            self.url.borrow().query().unwrap_or("")
        )
    }
}

fn with_url_mut<R>(params: &UrlSearchParams, f: impl FnOnce(&mut url::Url) -> R) -> JsResult<R> {
    let mut u = params.url.borrow_mut();
    Ok(f(&mut u))
}

fn read_pairs(params: &UrlSearchParams) -> JsResult<Vec<(String, String)>> {
    with_url_mut(params, |u| {
        u.query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect()
    })
}
fn write_pairs(params: &UrlSearchParams, pairs: &[(String, String)]) -> JsResult<()> {
    with_url_mut(params, |u| {
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

#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::interceptor::js::tests::setup;
    use boa_engine::Source;

    #[test]
    fn constructor_from_string_parses_pairs() {
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
    fn constructor_strips_leading_question_mark() {
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
    fn append_adds_pairs_and_preserves_existing() {
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
    fn get_returns_null_when_missing() {
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
    fn to_string_roundtrip() {
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
