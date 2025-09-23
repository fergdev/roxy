/// <reference path="../../script_libs/js/index.d.ts" />
/** @type {Extension} */
const query_set = {
  request(flow) {
    if (flow.request.url.searchParams.get("foo") == "bar") {
      flow.request.url.searchParams.set("foo", "baz");
    }
  }
};
globalThis.extensions = [query_set];
