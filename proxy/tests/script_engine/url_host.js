/// <reference path="../../script_libs/js/index.d.ts" />
/** @type {Extension} */
const url_host = {
  request(flow) {
    if (flow.request.url.host = "localhost:1234") {
      flow.request.url.host = "example.com:4321"
    }
  },
};
globalThis.extensions = [url_host]
