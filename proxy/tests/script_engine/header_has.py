from roxy import Extension


class AppendHeader(Extension):
    def request(self, flow):
        if flow.request.headers.has("X-Header1"):
            flow.request.body.text = "has"

    def response(self, flow):
        if flow.response.headers.has("X-Header1"):
            flow.response.body.text = "has"


Extensions = [AppendHeader()]
