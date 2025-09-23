/// <reference path="../../script_libs/js/index.d.ts" />
/** @type {Extension} */
const bodySet = {
  request(flow) {
    flow.request.body.text = "rewrite request";
  },
  response(flow) {
    flow.response.body.text = "rewrite response";
  }
};
globalThis.extensions = [bodySet];
