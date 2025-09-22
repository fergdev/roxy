/// <reference path="../../script_libs/js/index.d.ts" />
/** @type {Extension} */
const reqSetMethod = {
  request(flow) {
    if (flow.request.method == "GET") {
      flow.request.method = "POST";
    }
  }
};
globalThis.extensions = [reqSetMethod];
