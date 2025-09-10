class SetRespBody:
    def request(self, flow):
        flow.response.status = 404


Extensions = [SetRespBody()]
