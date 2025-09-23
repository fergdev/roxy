from roxy import Flow, Extension


class ClearBody(Extension):
    def request(self, flow: Flow):
        if not flow.request.body:
            flow.request.body.text = "empty request"

    def response(self, flow: Flow):
        if not flow.response.body:
            flow.response.body.text = "empty response"


Extensions = [ClearBody()]
