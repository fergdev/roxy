from roxy import Extension, Method


class ChangeMethod(Extension):
    def request(self, flow):
        print(flow.request.method)
        if flow.request.method == Method.GET:
            print("setting")
            flow.request.method = Method.POST
        else:
            print("not settings")

        print(flow.request.method)


Extensions = [ChangeMethod()]
