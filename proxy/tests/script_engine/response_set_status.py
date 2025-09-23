from roxy import Extension, Status


class SetStatus(Extension):
    def response(self, flow):
        flow.response.status = Status.NOT_FOUND


Extensions = [SetStatus()]
