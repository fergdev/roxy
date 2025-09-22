/// <reference path="../../script_libs/js/roxy.d.ts" />
/** @type {Extension} */
const url_port = {
  request(flow) {
    if (flow.request.url.port == 1234) {
      flow.request.url.port = 8080
    }
  },
};
globalThis.extensions = [url_port];
