
/// <reference path="../../script_libs/js/index.d.ts" />
/** @type {Extension} */
const url_path = {
  request(flow) {
    if (flow.request.url.path == "/some/path") {
      flow.request.url.path = "/another/path";
    }
  }
};
globalThis.extensions = [url_path]
