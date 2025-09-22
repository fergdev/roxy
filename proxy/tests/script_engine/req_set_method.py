from roxy import Extension, Method


class ChangeMethod(Extension):
    def request(self, flow):
        if flow.request.method == Method.GET:
            flow.request.method = Method.POST


Extensions = [ChangeMethod()]
