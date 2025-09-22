/// <reference path="../../script_libs/js/roxy.d.ts" />
/** @type {Extension} */
const requestSetRespStatus = {
  request(flow) {
    flow.response.status = 404
  }
};
globalThis.extensions = [requestSetRespStatus];
