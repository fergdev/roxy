/// <reference path="../../script_libs/js/roxy.d.ts" />
/** @type {Extension} */
const bodySub = {
  request(flow) {
    flow.request.body.text = flow.request.body.text.replace("replaceme", "gone");
  },
  response(flow) {
    flow.response.body.text = flow.response.body.text.replace("to_go", "it_went");
  }
};
globalThis.extensions = [bodySub];
