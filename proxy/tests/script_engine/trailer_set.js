/// <reference path="../../script_libs/js/roxy.d.ts" />
/** @type {Extension} */
const trailer_set = {
  request(flow) {
    flow.request.trailers.set("X-trailer1", "request");
  },
  response(flow) {
    flow.response.trailers.set("X-trailer1", "response");
  }
};
globalThis.extensions = [trailer_set];
