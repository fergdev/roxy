from roxy import Flow, Extension


class ChangeBody(Extension):
    def __init__(self, index: int) -> None:
        self.index = index

    def request(self, flow: Flow) -> None:
        t = flow.request.body.text or ""
        flow.request.body.text = f"{t} request{self.index}"

    def response(self, flow: Flow) -> None:
        t = flow.response.body.text or ""
        flow.response.body.text = f"{t} response{self.index}"


class ChangeBodyVoid(Exception):
    def __init__(self, index):
        self.index = index


Extensions = [ChangeBody(1), ChangeBodyVoid(99), ChangeBody(2)]
