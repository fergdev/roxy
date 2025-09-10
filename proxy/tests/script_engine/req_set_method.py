class ChangeMethod:
    def request(self, flow):
        flow.request.method = "POST"


Extensions = [ChangeMethod()]
