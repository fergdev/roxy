class ClearQuery:
    def request(self, flow):
        if flow.request.url.search_params["foo"] == "bar":
            flow.request.url.search_params.clear()


Extensions = [ClearQuery()]
