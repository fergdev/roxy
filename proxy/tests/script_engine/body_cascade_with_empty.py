class ChangeBody:
    def __init__(self, index):
        self.index = index

    def request(self, flow):
        flow.request.body.text = flow.request.body.text + " request" + str(self.index)

    def response(self, flow):
        flow.response.body.text = (
            flow.response.body.text + " response" + str(self.index)
        )


class ChangeBodyVoid:
    def __init__(self, index):
        self.index = index


Extensions = [ChangeBody(1), ChangeBodyVoid(99), ChangeBody(2)]
