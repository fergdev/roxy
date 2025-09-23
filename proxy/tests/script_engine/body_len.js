/// <reference path="../../script_libs/js/index.d.ts" />
/** @type {Extension} */
const bodyLen = {
  request(flow) {
    if (flow.request.body.length == 10) {
      flow.request.body.text = "len is 10 request";
    }
  },
  response(flow) {
    if (flow.response.body.length == 10) {
      flow.response.body.text = "len is 10 response";
    }
  }
};
globalThis.extensions = [bodyLen];
