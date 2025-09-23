/// <reference path="../../script_libs/js/index.d.ts" />
/** @type {Extension} */
const url_scheme = {
  request(flow) {
    console.log(flow.request.url.protocol + " " + Protocol.HTTP);
    if (flow.request.url.protocol == Protocol.HTTP) {
      flow.request.url.protocol = Protocol.HTTPS;
      console.log("set" + flow.request.url.protocol);
    }
  },
};
globalThis.extensions = [url_scheme];
