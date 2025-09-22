from roxy import Extension


class DeleteHeader(Extension):
    def request(self, flow):
        flow.request.headers.delete("X-Header1")
        flow.request.headers["X-Header2"] = None
        del flow.request.headers["X-header3"]

    def response(self, flow):
        flow.response.headers.delete("X-Header1")
        flow.response.headers["X-Header2"] = None
        del flow.response.headers["X-header3"]


Extensions = [DeleteHeader()]
