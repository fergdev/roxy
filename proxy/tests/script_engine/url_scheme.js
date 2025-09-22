/// <reference path="../../script_libs/js/roxy.d.ts" />
/** @type {Extension} */
const url_scheme = {
  request(flow) {
    if (flow.request.url.protocol == "http") {
      flow.request.url.protocol = "https";
    }
  },
};
globalThis.extensions = [url_scheme];
