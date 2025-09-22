class ChangeMethod:
    def request(self, flow):
        if flow.request.method == "GET":
            flow.request.method = "POST"


Extensions = [ChangeMethod()]
