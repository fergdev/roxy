from roxy import Extension


class AppendQuery(Extension):
    def request(self, flow):
        if flow.request.url.search_params["foo"] == "bar":
            flow.request.url.search_params.append("foo", "baz")


Extensions = [AppendQuery()]
