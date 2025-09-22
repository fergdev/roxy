from roxy import Flow, Extension


class ClearBody(Extension):
    def request(self, flow: Flow):
        flow.request.body.clear()

    def response(self, flow: Flow):
        flow.response.body.clear()


Extensions = [ClearBody()]
