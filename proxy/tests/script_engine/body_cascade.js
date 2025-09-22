/// <reference path="../../script_libs/js/roxy.d.ts" />

/**
 * Make a BodyCascade extension that appends the given id.
 * @param {number} id
 * @returns {Extension}
 */
function makeBodyCascade(id) {
  return {
    request(flow) {
      flow.request.body.text = (flow.request.body.text ?? "") + " request" + id;
    },
    response(flow) {
      flow.response.body.text = (flow.response.body.text ?? "") + " response" + id;
    }
  };
}

globalThis.extensions = [
  makeBodyCascade(1),
  makeBodyCascade(2),
];
