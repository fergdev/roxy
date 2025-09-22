/// <reference path="../../script_libs/js/roxy.d.ts" />
/** @type {Extension} */
const headerToString = {
  request(flow) {
    flow.request.body.text = flow.request.headers.toString();
  },
  response(flow) {
    flow.response.body.text = flow.response.headers.toString();
  }
};
globalThis.extensions = [headerToString];
