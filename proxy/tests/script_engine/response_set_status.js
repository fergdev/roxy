/// <reference path="../../script_libs/js/index.d.ts" />
/** @type {Extension} */
const responseSetStatus = {
  response(flow) {
    flow.response.status = Status.NOT_FOUND;
  }
};
globalThis.extensions = [responseSetStatus];
