from roxy import Extension


class SubBody(Extension):
    def request(self, flow):
        flow.request.body.text = flow.request.body.text.replace("replaceme", "gone")

    def response(self, flow):
        flow.response.body.text = flow.response.body.text.replace("to_go", "it_went")


Extensions = [SubBody()]
