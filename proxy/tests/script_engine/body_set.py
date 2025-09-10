class ChangeBody:
    def request(self, flow):
        flow.request.body.text = "rewrite request"

    def response(self, flow):
        flow.response.body.text = "rewrite response"


Extensions = [ChangeBody()]
