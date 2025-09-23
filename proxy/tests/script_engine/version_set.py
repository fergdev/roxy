from roxy import Extension, Version


class SetVersion(Extension):
    def request(self, flow):
        flow.request.version = Version.HTTP3

    def response(self, flow):
        flow.response.version = Version.HTTP3


Extensions = [SetVersion()]
