from roxy import Flow, Extension


class ChangeBody(Extension):
    def __init__(self, index: int):
        self.index = index

    def request(self, flow: Flow) -> None:
        t = flow.request.body.text or ""
        flow.request.body.text = f"{t} request{self.index}"

    def response(self, flow: Flow) -> None:
        t = flow.response.body.text or ""
        flow.response.body.text = f"{t} response{self.index}"


Extensions = [ChangeBody(1), ChangeBody(2)]
