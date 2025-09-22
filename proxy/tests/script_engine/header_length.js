/// <reference path="../../script_libs/js/roxy.d.ts" />
/** @type {Extension} */
const header_length = {
  request(flow) {
    console.log(flow.request.headers.length);
    if (flow.request.headers.length == 12) {
      flow.request.headers.clear();
    }
  },
  response(flow) {
    console.log(flow.request.headers.length);
    if (flow.response.headers.length == 12) {
      flow.response.headers.clear();
    }
  }
};
globalThis.extensions = [header_length];
