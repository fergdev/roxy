/// <reference path="../../script_libs/js/roxy.d.ts" />
/** @type {Extension} */
const queryClear = {
  request(flow) {
    if (flow.request.url.searchParams.get("foo") == "bar") {
      flow.request.url.searchParams.clear();
    }
  }
};
globalThis.extensions = [queryClear];
