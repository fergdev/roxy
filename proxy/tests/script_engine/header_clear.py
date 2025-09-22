from roxy import Extension


class ClearHeader(Extension):
    def request(self, flow):
        flow.request.headers.clear()

    def response(self, flow):
        flow.response.headers.clear()


Extensions = [ClearHeader()]
