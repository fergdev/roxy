/// <reference path="../../script_libs/js/index.d.ts" />
/** @type {Extension} */
const resp_set_body_based_on_req = {
  response(flow) {
    if (flow.request.url.host === "example.com") {
      flow.response.body.text = "intercepted"
    }
  }
};
globalThis.extensions = [resp_set_body_based_on_req];
