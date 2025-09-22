/// <reference path="../../script_libs/js/roxy.d.ts" />

/** @type {Extension} */
const bodyClear = {
  request(flow) {
    flow.request.body.clear();
  },
  response(flow) {
    flow.response.body.clear();
  }
}
globalThis.extensions = [bodyClear];
