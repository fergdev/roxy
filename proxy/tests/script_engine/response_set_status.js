/// <reference path="../../script_libs/js/roxy.d.ts" />
/** @type {Extension} */
const responseSetStatus = {
  response(flow) {
    flow.response.status = 404;
  }
};
globalThis.extensions = [responseSetStatus];
