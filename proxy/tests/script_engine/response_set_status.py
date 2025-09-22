from roxy import Extension


class SetStatus(Extension):
    def response(self, flow):
        flow.response.status = 404


Extensions = [SetStatus()]
