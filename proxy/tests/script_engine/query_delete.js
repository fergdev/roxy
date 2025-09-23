/// <reference path="../../script_libs/js/index.d.ts" />
/** @type {Extension} */
const query_delete = {
  request(flow) {
    if (flow.request.url.searchParams.get("foo") == "bar") {
      flow.request.url.searchParams.delete("foo")
    }
  }
};
globalThis.extensions = [query_delete];
