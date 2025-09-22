/// <reference path="../../script_libs/js/index.d.ts" />
/** @type {Extension} */
const trailer_has = {
  request(flow) {
    if (flow.request.trailers.has("X-trailer1")) {
      flow.request.body.text = "has"
    }
  },
  response(flow) {
    if (flow.response.trailers.has("X-trailer1")) {
      flow.response.body.text = "has"
    }
  }
};
globalThis.extensions = [trailer_has];
