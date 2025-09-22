/// <reference path="../../script_libs/js/index.d.ts" />
/** @type {Extension} */
const void_ext = {
  start() { },
  stop() { },
  request(flow) { },
  response(flow) { }
};
globalThis.extensions = [void_ext];
