/// <reference path="../../script_libs/js/index.d.ts" />
/** @type {Extension} */
const headerAppend = {
  request(flow) {
    flow.request.headers.append("X-Header1", "request");
    flow.request.headers.append("X-Header9", "request");
  },
  response(flow) {
    flow.response.headers.append("X-Header1", "response");
    flow.response.headers.append("X-Header9", "response");
  }
}
globalThis.extensions = [headerAppend];
