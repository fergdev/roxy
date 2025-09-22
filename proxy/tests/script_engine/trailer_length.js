/// <reference path="../../script_libs/js/index.d.ts" />
/** @type {Extension} */
const trailer_has = {
  request(flow) {
    const t = flow.request.trailers;
    if (t.length == 12) {
      t.clear();
    }
  },
  response(flow) {
    const t = flow.response.trailers;
    if (t.length == 12) {
      t.clear();
    }
  }
}
globalThis.extensions = [trailer_has];
