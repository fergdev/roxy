from roxy import Extension, Status


class SetRespBody(Extension):
    def request(self, flow):
        flow.response.status = Status.NOT_FOUND


Extensions = [SetRespBody()]
