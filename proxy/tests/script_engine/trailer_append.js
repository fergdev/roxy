/// <reference path="../../script_libs/js/index.d.ts" />
/** @type {Extension} */
const trailerAppend = {
  request(flow) {
    flow.request.trailers.append("X-Trailer1", "request");
    flow.request.trailers.append("X-Trailer9", "request");
  },
  response(flow) {
    flow.response.trailers.append("X-Trailer1", "response");
    flow.response.trailers.append("X-Trailer9", "response");
  }
};
globalThis.extensions = [trailerAppend];
