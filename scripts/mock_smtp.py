import json, sys, time
from aiosmtpd.controller import Controller

OUT = sys.argv[1] if len(sys.argv) > 1 else "/tmp/hearth-mails.jsonl"


class Sink:
    async def handle_DATA(self, server, session, envelope):
        with open(OUT, "a") as f:
            f.write(json.dumps({"to": envelope.rcpt_tos, "data": envelope.content.decode("utf8", "replace")}) + "\n")
        return "250 OK"


Controller(Sink(), hostname="127.0.0.1", port=2525).start()
while True:
    time.sleep(3600)
