from roxy import Flow, Extension


class StartInvoked(Extension):
    def start(self):
        self.count = 10

    def request(self, flow):
        flow.request.body.text = str(self.count)
        self.count += 1

    def response(self, flow):
        flow.response.body.text = str(self.count)
        self.count += 1


Extensions = [StartInvoked()]
