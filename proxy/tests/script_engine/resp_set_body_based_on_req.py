from roxy import Extension


class SetRespBody(Extension):
    def response(self, flow):
        if flow.request.url.host == "example.com":
            flow.response.body.text = "intercepted"


Extensions = [SetRespBody()]
