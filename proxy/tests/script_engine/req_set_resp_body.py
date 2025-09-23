from roxy import Extension


class SetRespBody(Extension):
    def request(self, flow):
        flow.response.body.text = "early return"


Extensions = [SetRespBody()]
