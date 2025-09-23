
/// <reference path="../../script_libs/js/index.d.ts" />
/** @type {Extension} */
const url_hostname = {
  request(flow) {
    if (flow.request.url.hostname = "localhost") {
      flow.request.url.hostname = "example.com"
    }
  },
};
globalThis.extensions = [url_hostname];
