class TestNotify:
    def request(self, flow):
        notify(1, "hi")

    def response(self, flow):
        notify(2, "there")


Extensions = [TestNotify()]
