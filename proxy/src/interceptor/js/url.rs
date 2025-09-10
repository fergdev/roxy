use boa_engine::value::Convert;
use boa_engine::{Context, Finalize, JsData, JsResult, JsString, JsValue, Trace, js_error};
use boa_interop::{JsClass, js_class};
use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;

use crate::interceptor::js::query::UrlSearchParams;

#[derive(Debug, Clone, JsData, Trace, Finalize)]
#[boa_gc(unsafe_no_drop)]
pub(crate) struct Url(#[unsafe_ignore_trace] Rc<RefCell<url::Url>>);

impl Url {
    pub fn register(context: &mut Context) -> JsResult<()> {
        context.register_global_class::<Self>()?;
        Ok(())
    }

    fn js_new(Convert(ref url): Convert<String>, base: Option<&Convert<String>>) -> JsResult<Self> {
        if let Some(Convert(base)) = base {
            let base_url = url::Url::parse(base)
                .map_err(|e| js_error!(TypeError: "Failed to parse base URL: {}", e))?;
            if base_url.cannot_be_a_base() {
                return Err(js_error!(TypeError: "Base URL {} cannot be a base", base));
            }

            let url = base_url
                .join(url)
                .map_err(|e| js_error!(TypeError: "Failed to parse URL: {}", e))?;
            Ok(Self(Rc::new(RefCell::new(url))))
        } else {
            let url = url::Url::parse(url)
                .map_err(|e| js_error!(TypeError: "Failed to parse URL: {}", e))?;
            Ok(Self(Rc::new(RefCell::new(url))))
        }
    }
}

impl Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.borrow())
    }
}

impl From<url::Url> for Url {
    fn from(url: url::Url) -> Self {
        Self(Rc::new(RefCell::new(url)))
    }
}

impl From<Url> for url::Url {
    fn from(url: Url) -> url::Url {
        url.0.borrow().clone()
    }
}

js_class! {
    class Url as "URL" {
        property hash {
            fn get(this: JsClass<Url>) -> JsString {
                JsString::from(url::quirks::hash(&this.borrow().0.borrow()))
            }

            fn set(this: JsClass<Url>, value: Convert<String>) {
                url::quirks::set_hash(&mut this.borrow_mut().0.borrow_mut(), &value.0);
            }
        }

        property hostname {
            fn get(this: JsClass<Url>) -> JsString {
                JsString::from(url::quirks::hostname(&this.borrow().0.borrow()))
            }

            fn set(this: JsClass<Url>, value: Convert<String>) {
                let _ = url::quirks::set_hostname(&mut this.borrow_mut().0.borrow_mut(), &value.0);
            }
        }

        property host {
            fn get(this: JsClass<Url>) -> JsString {
                JsString::from(url::quirks::host(&this.borrow().0.borrow()))
            }

            fn set(this: JsClass<Url>, value: Convert<String>) {
                let _ = url::quirks::set_host(&mut this.borrow_mut().0.borrow_mut(), &value.0);
            }
        }

        property href {
            fn get(this: JsClass<Url>) -> JsString {
                JsString::from(url::quirks::href(&this.borrow().0.borrow()))
            }

            fn set(this: JsClass<Url>, value: Convert<String>) -> JsResult<()> {
                url::quirks::set_href(&mut this.borrow_mut().0.borrow_mut(), &value.0)
                    .map_err(|e| js_error!(TypeError: "Failed to set href: {}", e))
            }
        }

        property origin {
            fn get(this: JsClass<Url>) -> JsString {
                JsString::from(url::quirks::origin(&this.borrow().0.borrow()))
            }
        }

        property password {
            fn get(this: JsClass<Url>) -> JsString {
                JsString::from(url::quirks::password(&this.borrow().0.borrow()))
            }

            fn set(this: JsClass<Url>, value: Convert<String>) {
                let _ = url::quirks::set_password(&mut this.borrow_mut().0.borrow_mut(), &value.0);
            }
        }

        property pathname {
            fn get(this: JsClass<Url>) -> JsString {
                JsString::from(url::quirks::pathname(&this.borrow().0.borrow()))
            }

            fn set(this: JsClass<Url>, value: Convert<String>) {
                let () = url::quirks::set_pathname(&mut this.borrow_mut().0.borrow_mut(), &value.0);
            }
        }

        property port {
            fn get(this: JsClass<Url>) -> JsString {
                JsString::from(url::quirks::port(&this.borrow().0.borrow()))
            }

            fn set(this: JsClass<Url>, value: Convert<JsString>) {
                let _ = url::quirks::set_port(&mut this.borrow_mut().0.borrow_mut(), &value.0.to_std_string_lossy());
            }
        }

        property protocol {
            fn get(this: JsClass<Url>) -> JsString {
                JsString::from(url::quirks::protocol(&this.borrow().0.borrow()))
            }

            fn set(this: JsClass<Url>, value: Convert<String>) {
                let _ = url::quirks::set_protocol(&mut this.borrow_mut().0.borrow_mut(), &value.0);
            }
        }

        property search {
            fn get(this: JsClass<Url>) -> JsString {
                JsString::from(url::quirks::search(&this.borrow().0.borrow()))
            }

            fn set(this: JsClass<Url>, value: Convert<String>) {
                url::quirks::set_search(&mut this.borrow_mut().0.borrow_mut(), &value.0);
            }
        }

        property search_params as "searchParams" {
            fn get(this: JsClass<Url>, context: &mut Context) -> JsResult<JsValue> {
                let weak = Rc::downgrade(&this.borrow().0);
                let params = UrlSearchParams { url: weak };
                let obj = UrlSearchParams::from_data(params, context)?;
                Ok(obj.into())
            }
        }

        property username {
            fn get(this: JsClass<Url>) -> JsString {
                JsString::from(this.borrow().0.borrow().username())
            }

            fn set(this: JsClass<Url>, value: Convert<String>) {
                let _ = this.borrow_mut().0.borrow_mut().set_username(&value.0);
            }
        }

        constructor(url: Convert<String>, base: Option<Convert<String>>) {
            Self::js_new(url, base.as_ref())
        }

        fn to_string as "toString"(this: JsClass<Url>) -> JsString {
            JsString::from(format!("{}", this.borrow().0.borrow()))
        }
    }
}
