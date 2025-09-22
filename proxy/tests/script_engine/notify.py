from roxy import Extension, notify


class TestNotify(Extension):
    def request(self, flow):
        notify(1, "hi")

    def response(self, flow):
        notify(2, "there")


Extensions = [TestNotify()]
