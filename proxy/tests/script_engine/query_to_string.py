from roxy import Extension


class QueryToString(Extension):
    def request(self, flow):
        flow.request.body.text = str(flow.request.url.search_params)


Extensions = [QueryToString()]
