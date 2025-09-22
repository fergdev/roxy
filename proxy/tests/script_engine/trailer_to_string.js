
/// <reference path="../../script_libs/js/roxy.d.ts" />
/** @type {Extension} */
const trailer_to_string = {
  request(flow) {
    flow.request.body.text = flow.request.trailers.toString();
  },
  response(flow) {
    flow.response.body.text = flow.response.trailers.toString();
  }
};
globalThis.extensions = [trailer_to_string];
