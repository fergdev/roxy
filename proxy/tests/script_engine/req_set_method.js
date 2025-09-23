/// <reference path="../../script_libs/js/index.d.ts" />
/** @type {Extension} */
const reqSetMethod = {
  request(flow) {
    if (flow.request.method == Method.GET) {
      flow.request.method = Method.POST;
    }
  }
};
globalThis.extensions = [reqSetMethod];
