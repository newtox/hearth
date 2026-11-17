import email, json, os, re, sys, time, requests

BASE = (sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:8080").rstrip("/") + "/api"
MAILS = sys.argv[2] if len(sys.argv) > 2 else "/tmp/hearth-mails.jsonl"
S = str(int(time.time() * 1000))[-7:]
fails = 0


def check(name, cond, info=""):
    global fails
    print(("ok   " if cond else "FAIL ") + name, "" if cond else info)
    fails += 0 if cond else 1


def mails():
    if not os.path.exists(MAILS):
        return []
    return [json.loads(line) for line in open(MAILS)]


start = len(mails())
address = f"lea{S}@example.com"
tok = requests.post(BASE + "/auth/register", json={"username": "lea" + S, "email": address, "password": "geheim1234", "lang": "de"}).json()["token"]
auth = {"Authorization": "Bearer " + tok}
time.sleep(1.5)
welcome = mails()[start:]
check("verification mail sent", len(welcome) == 1 and welcome[0]["to"] == [address], welcome)
check("account starts unconfirmed", requests.get(BASE + "/users/@me", headers=auth).json().get("email_verified") is False)
check("unconfirmed account cannot post", requests.post(BASE + "/guilds", headers=auth, json={"name": "x"}).status_code == 400)
text = email.message_from_string(welcome[0]["data"]).get_payload(decode=True).decode() if welcome else ""
verify = re.search(r"/#/verify/([0-9a-f]{64})", text)
check("verification link works", verify is not None and requests.post(BASE + "/auth/verify", json={"token": verify.group(1)}).status_code == 200, text[:300])
check("account is confirmed", requests.get(BASE + "/users/@me", headers=auth).json().get("email_verified") is True)
check("confirmed account can post", requests.post(BASE + "/guilds", headers=auth, json={"name": "x"}).status_code == 200)
before = len(mails())
check("reset is offered", requests.get(BASE + "/config").json()["password_reset"] is True)
check("unknown account answers the same", requests.post(BASE + "/auth/forgot", json={"login": "niemand" + S}).status_code == 200)
r = requests.post(BASE + "/auth/forgot", json={"login": address, "lang": "de"})
time.sleep(1.5)
new = mails()[before:]
check("one mail sent", r.status_code == 200 and len(new) == 1 and new[0]["to"] == [address], new)
body = email.message_from_string(new[0]["data"]).get_payload(decode=True).decode() if new else ""
link = re.search(r"/#/reset/([0-9a-f]{64})", body)
check("mail is German and has a link", link is not None and "Passwort" in body, body[:300])
token = link.group(1) if link else ""
check("short password rejected", requests.post(BASE + "/auth/reset", json={"token": token, "password": "kurz"}).status_code == 400)
r = requests.post(BASE + "/auth/reset", json={"token": token, "password": "ganzneu123"})
check("reset works and signs in", r.status_code == 200 and requests.get(BASE + "/users/@me", headers={"Authorization": "Bearer " + r.json()["token"]}).status_code == 200, r.text)
check("old sessions signed out", requests.get(BASE + "/users/@me", headers={"Authorization": "Bearer " + tok}).status_code == 401)
check("link works only once", requests.post(BASE + "/auth/reset", json={"token": token, "password": "nochmal123"}).status_code == 400)
check("login with new password", requests.post(BASE + "/auth/login", json={"login": address, "password": "ganzneu123"}).status_code == 200)
print(fails, "failed")
sys.exit(1 if fails else 0)
