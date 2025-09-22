/// <reference path="../../script_libs/js/roxy.d.ts" />
/** @type {Extension} */
const notify = {
  request() {
    globalThis.notify(1, "hi")
  },
  response() {
    globalThis.notify(2, "there")
  }
}
globalThis.extensions = [notify];
