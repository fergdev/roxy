/// <reference path="../../script_libs/js/roxy.d.ts" />
/** @type {Extension} */
const reqSetRespBody = {
  request(flow) {
    flow.response.body.text = "early return"
  }
};
globalThis.extensions = [reqSetRespBody];
