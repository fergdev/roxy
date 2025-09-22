/// <reference path="../../script_libs/js/index.d.ts" />
/** @type {Extension} */
const headerAppend = {
  request(flow) {
    flow.request.headers.clear();
  },
  response(flow) {
    flow.response.headers.clear();
  }
};
globalThis.extensions = [headerAppend];
