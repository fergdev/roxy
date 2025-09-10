use boa_engine::value::Convert;
use boa_engine::{Context, Finalize, JsData, JsResult, JsString, JsValue, Trace, js_error};
use boa_interop::{JsClass, js_class};
use std::cell::RefCell;
use std::rc::Weak;
use url::form_urlencoded::Serializer;

#[derive(Debug, Clone, JsData, Trace, Finalize)]
#[boa_gc(unsafe_no_drop)]
pub(crate) struct UrlSearchParams {
    #[unsafe_ignore_trace]
    pub(crate) url: Weak<RefCell<url::Url>>,
}

impl UrlSearchParams {
    pub fn register(context: &mut Context) -> JsResult<()> {
        context.register_global_class::<Self>()?;
        Ok(())
    }
    fn with_url_mut<R>(&self, f: impl FnOnce(&mut url::Url) -> R) -> JsResult<R> {
        let rc = self
            .url
            .upgrade()
            .ok_or_else(|| js_error!(Error: "URL object is gone"))?;
        let mut u = rc.borrow_mut();
        Ok(f(&mut u))
    }

    fn read_pairs(&self) -> JsResult<Vec<(String, String)>> {
        self.with_url_mut(|u| {
            u.query_pairs()
                .map(|(k, v)| (k.into_owned(), v.into_owned()))
                .collect()
        })
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
        constructor() {
            Err(js_error!(TypeError: "Illegal constructor"))
        }

        fn append(this: JsClass<UrlSearchParams>, key: Convert<String>, value: Convert<String>) -> JsResult<()> {
            let mut pairs = this.borrow().read_pairs()?;
            pairs.push((key.0.to_owned(), value.0.to_owned()));
            this.borrow().write_pairs(&pairs)
        }

        fn set(this: JsClass<UrlSearchParams>, key: Convert<String>, value: Convert<String>) -> JsResult<()> {
            let mut pairs = this.borrow().read_pairs()?;
            let k = key.0.to_owned();
            let v = value.0.to_owned();
            let mut found = false;
            for (kk, vv) in pairs.iter_mut() {
                if *kk == k {
                    if !found { *vv = v.clone(); found = true; }
                    else { *vv = String::new(); }
                }
            }
            if !found { pairs.push((k.clone(), v)); }
            pairs.retain(|(kk, vv)| !(kk == &k && vv.is_empty()));
            this.borrow().write_pairs(&pairs)
        }

        fn get(this: JsClass<UrlSearchParams>, key: Convert<String>) -> JsResult<JsValue> {
            for (k, v) in this.borrow().read_pairs()? {
                if k == key.0 { return Ok(JsValue::from(JsString::from(v))); }
            }
            Ok(JsValue::null())
        }

        fn has(this: JsClass<UrlSearchParams>, key: Convert<String>) -> JsResult<bool> {
            Ok(this.borrow().read_pairs()?.iter().any(|(k, _)| *k == key.0))
        }

        fn delete(this: JsClass<UrlSearchParams>, key: Convert<String>) -> JsResult<()> {
            let mut pairs = this.borrow().read_pairs()?;
            pairs.retain(|(k, _)| *k != key.0);
            this.borrow().write_pairs(&pairs)
        }

        fn to_string as "toString"(this: JsClass<UrlSearchParams>) -> JsResult<JsString> {
            let mut s = Serializer::new(String::new());
            for (k, v) in this.borrow().read_pairs()? { s.append_pair(&k, &v); }
            Ok(JsString::from(s.finish()))
        }

        // TODO: implement iterator protocol [Symbol.iterator]() → entries()
        // and `.keys()`, `.values()`, `.entries()` if your engine supports it.
    }
}
