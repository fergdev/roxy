/// <reference path="../../script_libs/js/roxy.d.ts" />
/** @type {Extension} */
const url_username = {
  request(flow) {
    if (flow.request.url.username == "dave") {
      flow.request.url.username = "damo";
    }
  },
};
globalThis.extensions = [url_username];
