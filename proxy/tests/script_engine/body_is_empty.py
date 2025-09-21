class ClearBody:
    def request(self, flow):
        if flow.request.body.is_empty:
            flow.request.body.text = "empty request"

    def response(self, flow):
        if flow.response.body.is_empty:
            flow.response.body.text = "empty response"


Extensions = [ClearBody()]
