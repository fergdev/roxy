import json
from roxy import Extension


class StopInvoked(Extension):
    def start(self):
        self.count = 10
        self.out_file = ""

    def request(self, flow):
        flow.request.body.text = str(self.count)
        self.count += 1

    def response(self, flow):
        self.out_file = flow.response.body.text
        flow.response.body.text = str(self.count)
        self.count += 1

    def stop(self):
        with open(self.out_file, "w") as f:
            json.dump({"stopped": True, "count": self.count}, f)


Extensions = [StopInvoked()]
