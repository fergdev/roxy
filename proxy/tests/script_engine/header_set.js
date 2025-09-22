/// <reference path="../../script_libs/js/roxy.d.ts" />
/** @type {Extension} */
const headerSet = {
  request(flow) {
    flow.request.headers.set("X-Header1", "request");
  },
  response(flow) {
    flow.response.headers.set("X-Header1", "response");
  }
}

globalThis.extensions = [headerSet];
