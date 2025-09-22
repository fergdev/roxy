/// <reference path="../../script_libs/js/roxy.d.ts" />
/** @type {Extension} */

let count = 0
let out_file = ""
const stopInvoked = {
  start() {
    count = 10
  },
  request(flow) {
    flow.request.body.text = count;
    count += 1;
  },
  response(flow) {
    out_file = flow.response.body.text;
    flow.response.body.text = count;
  },
  stop() {
    globalThis.writeFile(
      out_file,
      JSON.stringify({ stopped: true, count: this.count })
    );
  }
}
globalThis.extensions = [stopInvoked];
