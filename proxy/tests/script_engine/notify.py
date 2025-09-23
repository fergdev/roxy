from roxy import Extension, notify, Flow


class TestNotify(Extension):
    def request(self, flow: Flow):
        notify(1, "hi")

    def response(self, flow):
        notify(2, "there")


Extensions = [TestNotify()]
