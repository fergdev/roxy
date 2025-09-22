/// <reference path="../../script_libs/js/roxy.d.ts" />
/** @type {Extension} */
const trailer_clear = {
  request(flow) {
    flow.request.trailers.clear();
  },
  response(flow) {
    flow.response.trailers.clear();
  }
}
globalThis.extensions = [trailer_clear];
