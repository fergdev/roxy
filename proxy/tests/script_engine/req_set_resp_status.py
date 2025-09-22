from roxy import Extension


class SetRespBody(Extension):
    def request(self, flow):
        flow.response.status = 404


Extensions = [SetRespBody()]
