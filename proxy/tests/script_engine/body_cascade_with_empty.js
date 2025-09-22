globalThis.extensions = [
  {
    request(flow) {
      flow.request.body.text = flow.request.body.text + " request1";
    },
    response(flow) {
      flow.response.body.text = flow.response.body.text + " response1";
    }
  },
  {},
  {
    request(flow) {
      flow.request.body.text = flow.request.body.text + " request2";
    },
    response(flow) {
      flow.response.body.text = flow.response.body.text + " response2";
    }
  }
];
