/// <reference path="../../script_libs/js/index.d.ts" />
/** @type {Extension} */
let count = 0;
const start_invoked = {
  start() {
    count = 10
  },
  request(flow) {
    flow.request.body.text = count;
    count += 1;
  },
  response(flow) {
    flow.response.body.text = count;
  }
}
globalThis.extensions = [start_invoked];
