/// <reference path="../../script_libs/js/index.d.ts" />
/** @type {Extension} */
const version_set = {
  request(flow) {
    flow.request.version = Version.HTTP3_0
  },
  response(flow) {
    flow.response.version = Version.HTTP3_0
  }
};
globalThis.extensions = [version_set];
