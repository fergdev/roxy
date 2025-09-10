class SetStatus:
    def response(self, flow):
        flow.response.status = 404


Extensions = [SetStatus()]
