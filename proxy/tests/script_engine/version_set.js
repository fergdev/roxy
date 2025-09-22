/// <reference path="../../script_libs/js/index.d.ts" />
/** @type {Extension} */
const version_set = {
  request(flow) {
    flow.request.version = "HTTP/3.0";
  },
  response(flow) {
    flow.response.version = "HTTP/3.0";
  }
};
globalThis.extensions = [version_set];
