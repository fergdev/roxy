class ClearBody:
    def request(self, flow):
        if not flow.request.body:
            flow.request.body.text = "empty request"

    def response(self, flow):
        if not flow.response.body:
            flow.response.body.text = "empty response"


Extensions = [ClearBody()]
