/// <reference path="../../script_libs/js/index.d.ts" />
/** @type {Extension} */
const requestSetRespStatus = {
  request(flow) {
    flow.response.status = Status.NOT_FOUND;
  }
};
globalThis.extensions = [requestSetRespStatus];
