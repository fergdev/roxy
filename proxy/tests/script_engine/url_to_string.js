/// <reference path="../../script_libs/js/index.d.ts" />
/** @type {Extension} */
const url_to_string = {
  request(flow) {
    console.log(flow.request.url.toString())
    flow.request.body.text = flow.request.url.toString();
  }
};
globalThis.extensions = [url_to_string];
