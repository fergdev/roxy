/// <reference path="../../script_libs/js/roxy.d.ts" />
/** @type {Extension} */
const url_password = {
  request(flow) {
    if (flow.request.url.password == "1234") {
      flow.request.url.password = "abcd";
    }
  },
};
globalThis.extensions = [url_password]
