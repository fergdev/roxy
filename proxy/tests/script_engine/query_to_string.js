/// <reference path="../../script_libs/js/roxy.d.ts" />
/** @type {Extension} */
const query_to_string = {
  request(flow) {
    flow.request.body.text = flow.request.url.searchParams.toString();
  }
};
globalThis.extensions = [query_to_string];
