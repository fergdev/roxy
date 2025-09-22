/// <reference path="../../script_libs/js/roxy.d.ts" />
/** @type {Extension} */
const bodyIsEmpty = {
  request(flow) {
    if (flow.request.body.isEmpty) {
      flow.request.body.text = "empty request";
    }
  },
  response(flow) {
    if (flow.response.body.isEmpty) {
      flow.response.body.text = "empty response";
    }
  }
};
globalThis.extensions = [bodyIsEmpty];
