class ClearBody:
    def request(self, flow):
        flow.request.body.clear()

    def response(self, flow):
        flow.response.body.clear()


Extensions = [ClearBody()]
