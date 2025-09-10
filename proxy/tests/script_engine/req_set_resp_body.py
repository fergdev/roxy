class SetRespBody:
    def request(self, flow):
        flow.response.body.text = "early return"


Extensions = [SetRespBody()]
