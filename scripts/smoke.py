import asyncio, json, sys, time, requests, websockets

BASE = sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:8080"
WS = BASE.replace("http", "ws") + "/gateway"
S = str(int(time.time()))
fails = 0

def check(name, cond, info=""):
    global fails
    print(("ok   " if cond else "FAIL ") + name, info if not cond else "")
    fails += 0 if cond else 1

def call(method, path, token=None, body=None, files=None):
    h = {"Authorization": f"Bearer {token}"} if token else {}
    r = requests.request(method, BASE + "/api" + path, headers=h, json=body, files=files)
    try:
        return r.status_code, r.json()
    except Exception:
        return r.status_code, None

def user(name):
    s, b = call("POST", "/auth/register", body={"username": name + S, "email": f"{name}{S}@example.com", "password": "geheim1234"})
    assert s == 200, (s, b)
    return b["token"], call("GET", "/users/@me", b["token"])[1]["id"]

async def gateway(token):
    ws = await websockets.connect(WS, max_size=None)
    await ws.send(json.dumps({"op": "identify", "token": token}))
    ready = json.loads(await ws.recv())
    return ws, ready

async def wait_for(ws, t, timeout=3, match=lambda d: True):
    end = time.time() + timeout
    while time.time() < end:
        try:
            ev = json.loads(await asyncio.wait_for(ws.recv(), end - time.time()))
        except asyncio.TimeoutError:
            break
        if ev["t"] == t and match(ev["d"]):
            return ev["d"]
    return None

async def main():
    a, aid = user("anna")
    b, bid = user("ben")
    c, cid_user = user("cora")

    check("register rejects duplicate", call("POST", "/auth/register", body={"username": "anna" + S, "email": "x@example.com", "password": "geheim1234"})[0] == 409)
    check("login wrong password", call("POST", "/auth/login", body={"login": "anna" + S, "password": "falsch123"})[0] == 400)
    check("login by email", call("POST", "/auth/login", body={"login": f"anna{S}@example.com", "password": "geheim1234"})[0] == 200)
    check("no token -> 401", call("GET", "/users/@me")[0] == 401)

    s, g = call("POST", "/guilds", a, {"name": "Testgilde"})
    check("create guild", s == 200 and len(g["channels"]) == 2, g)
    gid = g["id"]
    general = next(ch for ch in g["channels"] if ch["kind"] == "text")["id"]
    everyone = next(r for r in g["roles"] if r["is_default"])["id"]

    check("non-member cannot see guild", call("GET", f"/guilds/{gid}", b)[0] == 404)
    check("non-member cannot read channel", call("GET", f"/channels/{general}/messages", b)[0] == 404)

    s, inv = call("POST", f"/guilds/{gid}/invites", a, {})
    check("create invite", s == 200)
    check("invite preview", call("GET", f"/invites/{inv['code']}", b)[1]["name"] == "Testgilde")
    check("ben joins", call("POST", f"/invites/{inv['code']}", b)[0] == 200)
    check("cora joins", call("POST", f"/invites/{inv['code']}", c)[0] == 200)

    wa, ra = await gateway(a)
    wb, rb = await gateway(b)
    check("READY has guild", ra["t"] == "READY" and ra["d"]["guilds"][0]["id"] == gid)
    check("anna sees ben come online", await wait_for(wa, "PRESENCE_UPDATE", match=lambda d: d["user_id"] == bid and d["online"]) is not None)

    s, m = call("POST", f"/channels/{general}/messages", b, {"content": "Hallo zusammen"})
    check("send message", s == 200)
    ev = await wait_for(wa, "MESSAGE_CREATE")
    check("anna receives message live", ev and ev["content"] == "Hallo zusammen" and ev["author"]["id"] == bid)
    check("empty message rejected", call("POST", f"/channels/{general}/messages", b, {"content": "   "})[0] == 400)
    check("anna cannot edit ben's message", call("PATCH", f"/channels/{general}/messages/{m['id']}", a, {"content": "x"})[0] == 404)
    check("ben edits", call("PATCH", f"/channels/{general}/messages/{m['id']}", b, {"content": "Hallo!"})[1]["edited_at"] is not None)
    check("cora cannot delete ben's message", call("DELETE", f"/channels/{general}/messages/{m['id']}", c)[0] == 403)

    check("ben cannot create channel", call("POST", f"/guilds/{gid}/channels", b, {"name": "geheim"})[0] == 403)
    check("ben cannot create role", call("POST", f"/guilds/{gid}/roles", b, {"name": "x"})[0] == 403)
    check("ben cannot kick cora", call("DELETE", f"/guilds/{gid}/members/{cid_user}", b)[0] == 403)

    MANAGE_MESSAGES, MANAGE_CHANNELS, MANAGE_ROLES, KICK, ADMIN, VIEW = 4, 8, 16, 64, 2048, 1
    s, mod = call("POST", f"/guilds/{gid}/roles", a, {"name": "Mod", "color": "#C0392B", "permissions": MANAGE_MESSAGES | MANAGE_ROLES | KICK, "hoist": True})
    check("create role", s == 200 and mod["color"] == "#c0392b", mod)
    s, vip = call("POST", f"/guilds/{gid}/roles", a, {"name": "VIP"})
    check("role positions", call("GET", f"/guilds/{gid}", a)[1]["roles"][0]["name"] == "Mod")
    check("assign Mod to ben", call("PATCH", f"/guilds/{gid}/members/{bid}", a, {"roles": [mod["id"]]})[0] == 200)
    check("ben gets GUILD_REFRESH", await wait_for(wb, "GUILD_REFRESH") is not None)

    check("mod deletes others' message", call("POST", f"/channels/{general}/messages", c, {"content": "spam"})[0] == 200)
    last = call("GET", f"/channels/{general}/messages", b)[1][-1]
    check("history order", last["content"] == "spam")
    check("mod can delete", call("DELETE", f"/channels/{general}/messages/{last['id']}", b)[0] == 200)
    check("mod cannot grant admin", call("PATCH", f"/guilds/{gid}/roles/{vip['id']}", b, {"permissions": ADMIN})[0] == 403)
    check("mod cannot edit own top role", call("PATCH", f"/guilds/{gid}/roles/{mod['id']}", b, {"name": "Boss"})[0] == 403)
    check("mod can edit lower role", call("PATCH", f"/guilds/{gid}/roles/{vip['id']}", b, {"name": "Stammgast"})[0] == 200)
    check("mod cannot give himself Mod-level role to cora", call("PATCH", f"/guilds/{gid}/members/{cid_user}", b, {"roles": [mod["id"]]})[0] == 403)
    check("mod can give lower role", call("PATCH", f"/guilds/{gid}/members/{cid_user}", b, {"roles": [vip["id"]]})[0] == 200)
    check("mod cannot kick owner", call("DELETE", f"/guilds/{gid}/members/{aid}", b)[0] == 403)
    check("mod cannot reorder above himself", call("PUT", f"/guilds/{gid}/roles", b, [vip["id"], mod["id"]])[0] == 403)
    check("owner reorders", call("PUT", f"/guilds/{gid}/roles", a, [vip["id"], mod["id"]])[0] == 200)
    check("now cora outranks ben: kick refused", call("DELETE", f"/guilds/{gid}/members/{cid_user}", b)[0] == 403)
    check("owner reorders back", call("PUT", f"/guilds/{gid}/roles", a, [mod["id"], vip["id"]])[0] == 200)

    s, secret = call("POST", f"/guilds/{gid}/channels", a, {"name": "mod-stube"})
    check("create channel", s == 200)
    check("hide channel from everyone", call("PUT", f"/channels/{secret['id']}/overwrites/{everyone}", a, {"kind": "role", "allow": 0, "deny": VIEW})[0] == 200)
    check("allow Mod", call("PUT", f"/channels/{secret['id']}/overwrites/{mod['id']}", a, {"kind": "role", "allow": VIEW, "deny": 0})[0] == 200)
    names = lambda t: [ch["name"] for ch in call("GET", f"/guilds/{gid}", t)[1]["channels"]]
    check("cora does not see hidden channel", "mod-stube" not in names(c))
    check("ben sees hidden channel", "mod-stube" in names(b))
    check("cora cannot read hidden channel", call("GET", f"/channels/{secret['id']}/messages", c)[0] == 404)
    check("cora cannot post in hidden channel", call("POST", f"/channels/{secret['id']}/messages", c, {"content": "hi"})[0] == 404)
    wc, _ = await gateway(c)
    call("POST", f"/channels/{secret['id']}/messages", a, {"content": "nur für Mods"})
    check("ben receives hidden-channel message", await wait_for(wb, "MESSAGE_CREATE", match=lambda d: d["content"] == "nur für Mods") is not None)
    check("cora does not receive it", await wait_for(wc, "MESSAGE_CREATE", 1, match=lambda d: d["content"] == "nur für Mods") is None)

    s, dm = call("POST", "/users/@me/channels", b, {"user_id": cid_user})
    check("open DM", s == 200 and len(dm["recipients"]) == 2)
    check("DM is reused", call("POST", "/users/@me/channels", c, {"user_id": bid})[1]["id"] == dm["id"])
    check("DM message", call("POST", f"/channels/{dm['id']}/messages", b, {"content": "psst"})[0] == 200)
    check("cora gets DM live", await wait_for(wc, "MESSAGE_CREATE", match=lambda d: d["content"] == "psst") is not None)
    check("anna cannot read foreign DM", call("GET", f"/channels/{dm['id']}/messages", a)[0] == 404)
    d, did = user("dora")
    check("no DM without shared server", call("POST", "/users/@me/channels", d, {"user_id": aid})[0] == 403)

    png = bytes.fromhex("89504e470d0a1a0a0000000d49484452000000010000000108060000001f15c4890000000d49444154789c6360f8cfc0f01f0005050201a5c8a1c40000000049454e44ae426082")
    s, me = call("POST", "/users/@me/avatar", b, files={"file": ("a.png", png, "image/png")})
    check("avatar upload", s == 200 and me["avatar"].endswith(".png"), me)
    check("avatar served", requests.get(f"{BASE}/uploads/{me['avatar']}").content == png)
    check("html upload rejected", call("POST", "/users/@me/banner", b, files={"file": ("a.png", b"<html>", "image/png")})[0] == 400)
    s, me = call("PATCH", "/users/@me", b, {"display_name": "Ben der Barde", "bio": "Spielt Laute.", "accent_color": "#7755aa"})
    check("profile update", s == 200 and me["bio"] == "Spielt Laute.")
    check("profile visible", call("GET", f"/users/{bid}", a)[1]["display_name"] == "Ben der Barde")
    check("email not public", "email" not in call("GET", f"/users/{bid}", a)[1])

    txt = b"Hallo Welt, das ist eine Textdatei."
    s, att = call("POST", f"/channels/{general}/attachments", a, files={"file": ("bild.png", png, "image/png")})
    check("upload image attachment", s == 200 and att["is_image"], att)
    s, att2 = call("POST", f"/channels/{general}/attachments", a, files={"file": ("notiz.txt", txt, "text/plain")})
    check("upload file attachment", s == 200 and not att2["is_image"] and att2["file"].startswith("files/"), att2)
    s, am = call("POST", f"/channels/{general}/messages", a, {"content": "", "attachment_ids": [att["id"], att2["id"]]})
    check("message with attachments only", s == 200 and len(am["attachments"]) == 2, am)
    r = requests.get(f"{BASE}/uploads/{att2['file']}")
    check("file served as download", r.content == txt and "attachment" in r.headers.get("content-disposition", "") and r.headers.get("content-type") == "application/octet-stream", dict(r.headers))
    check("cora cannot reuse anna's attachment", call("POST", f"/channels/{general}/messages", c, {"content": "x", "attachment_ids": [att["id"]]})[0] == 400)
    check("html disguised as image stays a download", call("POST", f"/channels/{general}/attachments", a, files={"file": ("x.png", b"<script>alert(1)</script>", "image/png")})[1]["file"].startswith("files/"))

    check("react", call("POST", f"/channels/{general}/messages/{am['id']}/reactions", c, {"emoji": "👍", "on": True})[0] == 200)
    check("react custom", call("POST", f"/channels/{general}/messages/{am['id']}/reactions", a, {"emoji": "<:party:0192abcdef.png>", "on": True})[0] == 200)
    check("react rejects text", call("POST", f"/channels/{general}/messages/{am['id']}/reactions", c, {"emoji": "<b>hi</b>", "on": True})[0] == 400)
    hist = call("GET", f"/channels/{general}/messages", a)[1][-1]
    check("reactions in history", [r["emoji"] for r in hist["reactions"]] == ["👍", "<:party:0192abcdef.png>"] and hist["reactions"][0]["users"] == [cid_user], hist["reactions"])
    check("unreact", call("POST", f"/channels/{general}/messages/{am['id']}/reactions", c, {"emoji": "👍", "on": False})[0] == 200 and len(call("GET", f"/channels/{general}/messages", a)[1][-1]["reactions"]) == 1)
    check("delete message removes files", call("DELETE", f"/channels/{general}/messages/{am['id']}", a)[0] == 200 and requests.get(f"{BASE}/uploads/{att2['file']}").status_code == 404)

    r = requests.post(f"{BASE}/api/guilds/{gid}/emojis", headers={"Authorization": f"Bearer {a}"}, data={"name": "Party", "kind": "emoji"}, files={"file": ("p.png", png, "image/png")})
    check("create emoji", r.status_code == 200 and r.json()["name"] == "party", r.text)
    r2 = requests.post(f"{BASE}/api/guilds/{gid}/emojis", headers={"Authorization": f"Bearer {a}"}, data={"name": "winken", "kind": "sticker"}, files={"file": ("p.png", png, "image/png")})
    check("create sticker", r2.status_code == 200, r2.text)
    check("ben cannot create emoji", requests.post(f"{BASE}/api/guilds/{gid}/emojis", headers={"Authorization": f"Bearer {b}"}, data={"name": "nope"}, files={"file": ("p.png", png, "image/png")}).status_code == 403)
    check("emojis in snapshot", [e["kind"] for e in call("GET", f"/guilds/{gid}", c)[1]["emojis"]] == ["emoji", "sticker"])
    ext = "<:fremd:0192abcdef.png>"
    check("external emoji allowed by default", call("POST", f"/channels/{general}/messages", c, {"content": ext})[0] == 200)
    everyone_perms = next(x for x in call("GET", f"/guilds/{gid}", a)[1]["roles"] if x["is_default"])["permissions"]
    call("PATCH", f"/guilds/{gid}/roles/{everyone}", a, {"permissions": everyone_perms & ~(1 << 20)})
    check("external emoji blocked without permission", call("POST", f"/channels/{general}/messages", c, {"content": ext})[0] == 400)
    check("external reaction blocked", call("POST", f"/channels/{general}/messages/{m['id']}/reactions", c, {"emoji": ext, "on": True})[0] == 400)
    check("own server emoji still fine", call("POST", f"/channels/{general}/messages", c, {"content": f"<:party:{r.json()['file']}>"})[0] == 200)
    check("unicode emoji still fine", call("POST", f"/channels/{general}/messages/{m['id']}/reactions", c, {"emoji": "🎉", "on": True})[0] == 200)
    check("member rule re-allows", call("PUT", f"/channels/{general}/overwrites/{cid_user}", a, {"kind": "member", "allow": 1 << 20, "deny": 0})[0] == 200 and call("POST", f"/channels/{general}/messages", c, {"content": ext})[0] == 200)
    call("DELETE", f"/channels/{general}/overwrites/{cid_user}", a)
    call("PATCH", f"/guilds/{gid}/roles/{everyone}", a, {"permissions": everyone_perms})
    check("delete emoji", call("DELETE", f"/guilds/{gid}/emojis/{r.json()['id']}", a)[0] == 200 and requests.get(f"{BASE}/uploads/{r.json()['file']}").status_code == 404)

    check("mod without permission cannot timeout", call("PATCH", f"/guilds/{gid}/members/{cid_user}", b, {"timeout_minutes": 10})[0] == 403)
    check("owner times cora out", call("PATCH", f"/guilds/{gid}/members/{cid_user}", a, {"timeout_minutes": 10})[0] == 200)
    check("timed out cannot send", call("POST", f"/channels/{general}/messages", c, {"content": "hm"})[0] == 403)
    check("timed out cannot react", call("POST", f"/channels/{general}/messages/{m['id']}/reactions", c, {"emoji": "👍", "on": True})[0] == 403)
    check("timed out can read", call("GET", f"/channels/{general}/messages", c)[0] == 200)
    check("timeout visible", next(x for x in call("GET", f"/guilds/{gid}", a)[1]["members"] if x["id"] == cid_user)["timeout_until"] is not None)
    check("timeout lifted", call("PATCH", f"/guilds/{gid}/members/{cid_user}", a, {"timeout_minutes": 0})[0] == 200 and call("POST", f"/channels/{general}/messages", c, {"content": "wieder da"})[0] == 200)

    s, log = call("GET", f"/guilds/{gid}/audit", a)
    actions = [e["action"] for e in log]
    check("audit log", s == 200 and all(x in actions for x in ("role_create", "member_roles", "channel_create", "timeout", "message_delete", "emoji_create")), actions)
    check("audit log needs permission", call("GET", f"/guilds/{gid}/audit", c)[0] == 403)

    s, vc = call("POST", f"/guilds/{gid}/channels", a, {"name": "Lagerfeuer", "kind": "voice"})
    check("create voice channel", s == 200 and vc["kind"] == "voice")
    check("no text in voice channel", call("POST", f"/channels/{vc['id']}/messages", a, {"content": "x"})[0] == 400)
    s, v = call("POST", f"/channels/{vc['id']}/voice", b)
    if s == 503:
        print("skip voice (LIVEKIT_* not set)")
    else:
        import base64, hashlib, hmac, os
        h, p, sig = v["token"].split(".")
        pad = lambda x: x + "=" * (-len(x) % 4)
        claims = json.loads(base64.urlsafe_b64decode(pad(p)))
        good = base64.urlsafe_b64encode(hmac.new(os.environ.get("LIVEKIT_API_SECRET", "secret").encode(), f"{h}.{p}".encode(), hashlib.sha256).digest()).rstrip(b"=").decode()
        check("voice token", s == 200 and sig == good and claims["video"]["room"] == vc["id"] and claims["sub"] == bid and "screen_share" in claims["video"]["canPublishSources"], claims)
        check("VOICE_STATE event", await wait_for(wa, "VOICE_STATE", match=lambda d: d["user_id"] == bid and d["channel_id"] == vc["id"]) is not None)
        check("voice state in snapshot", call("GET", f"/guilds/{gid}", a)[1]["voice_states"] == [{"user_id": bid, "channel_id": vc["id"]}])
        check("text channel is no voice channel", call("POST", f"/channels/{general}/voice", b)[0] == 400)
        check("leave voice", call("DELETE", "/voice", b)[0] == 200 and call("GET", f"/guilds/{gid}", a)[1]["voice_states"] == [])
        call("POST", f"/channels/{vc['id']}/voice", b)
        await wb.close()
        check("disconnect leaves voice", await wait_for(wa, "VOICE_STATE", match=lambda d: d["user_id"] == bid and d["channel_id"] is None) is not None)
        wb, _ = await gateway(b)

    if call("GET", "/users/@me", a)[1]["is_admin"]:
        check("founder badge", call("GET", f"/users/{aid}", b)[1]["badges"][0]["name"] == "Founder")
        s, bd = call("POST", "/badges", a, {"name": "Tester" + S, "icon": "🧪", "description": "Testet", "color": "#22aa66"})
        check("create badge", s == 200, bd)
        check("non-admin cannot create badge", call("POST", "/badges", b, {"name": "x", "icon": "🧪"})[0] == 403)
        check("non-admin cannot grant", call("PUT", f"/users/{bid}/badges/{bd['id']}", b)[0] == 403)
        check("grant badge", call("PUT", f"/users/{bid}/badges/{bd['id']}", a)[0] == 200 and call("GET", f"/users/{bid}", c)[1]["badges"][0]["icon"] == "🧪")
        check("revoke badge", call("DELETE", f"/users/{bid}/badges/{bd['id']}", a)[0] == 200 and call("GET", f"/users/{bid}", c)[1]["badges"] == [])
        check("delete badge", call("DELETE", f"/badges/{bd['id']}", a)[0] == 200)
    else:
        print("skip badges (anna is not the first user of the instance)")
    check("badge list", len(call("GET", "/badges", b)[1]) >= 3)

    check("set activity", call("POST", "/users/@me/activity", c, {"kind": "playing", "name": "Tetris"})[0] == 200)
    ev = await wait_for(wa, "PRESENCE_UPDATE", match=lambda d: d["user_id"] == cid_user and d["activity"])
    check("activity broadcast", ev and ev["activity"]["name"] == "Tetris" and ev["activity"]["kind"] == "playing", ev)
    check("activity in snapshot", next(x for x in call("GET", f"/guilds/{gid}", a)[1]["members"] if x["id"] == cid_user)["activity"]["name"] == "Tetris")
    check("activity in profile", call("GET", f"/users/{cid_user}", a)[1]["activity"]["kind"] == "playing")
    check("invalid activity", call("POST", "/users/@me/activity", c, {"kind": "hacking", "name": "x"})[0] == 400)
    check("clear activity", call("DELETE", "/users/@me/activity", c)[0] == 200 and call("GET", f"/users/{cid_user}", a)[1]["activity"] is None)
    check("no streaming outside voice", call("POST", "/voice/stream", c, {"on": True})[0] == 200 and call("GET", f"/users/{cid_user}", a)[1]["streaming"] is False)
    if call("GET", "/users/@me", a)[1]["is_admin"]:
        s, bd = call("POST", "/badges", a, {"name": "Stern" + S, "icon": "svg:star"})
        check("badge with built-in icon", s == 200 and bd["icon"] == "svg:star", bd)
        check("unknown built-in icon rejected", call("POST", "/badges", a, {"name": "Nope" + S, "icon": "svg:nope"})[0] == 400)
        call("DELETE", f"/badges/{bd['id']}", a)
    check("spotify off by default", call("GET", "/spotify", a)[0] == 503 and call("GET", "/users/@me", a)[1]["spotify"] == {"available": False, "linked": False})

    s, pm = call("POST", f"/channels/{general}/messages", a, {"content": "", "poll": {"question": "Wann raiden?", "options": ["Freitag", "Samstag", "Sonntag"], "hours": 24}})
    check("create poll", s == 200 and len(pm["poll"]["options"]) == 3 and pm["poll"]["expires_at"], pm)
    o = [x["id"] for x in pm["poll"]["options"]]
    check("poll needs two options", call("POST", f"/channels/{general}/messages", a, {"poll": {"question": "x", "options": ["nur eine"]}})[0] == 400)
    check("vote", call("POST", f"/channels/{general}/messages/{pm['id']}/poll", c, {"option_ids": [o[1]]})[0] == 200)
    check("single choice enforced", call("POST", f"/channels/{general}/messages/{pm['id']}/poll", c, {"option_ids": [o[0], o[1]]})[0] == 400)
    check("change vote", call("POST", f"/channels/{general}/messages/{pm['id']}/poll", c, {"option_ids": [o[2]]})[0] == 200)
    got = next(x for x in call("GET", f"/channels/{general}/messages", a)[1] if x["id"] == pm["id"])["poll"]["options"]
    check("votes counted", [x["voters"] for x in got] == [[], [], [cid_user]], got)
    check("foreign option rejected", call("POST", f"/channels/{general}/messages/{pm['id']}/poll", c, {"option_ids": [aid]})[0] == 400)
    check("outsider cannot vote", call("POST", f"/channels/{general}/messages/{pm['id']}/poll", d, {"option_ids": [o[0]]})[0] == 404)
    s, pm2 = call("POST", f"/channels/{general}/messages", a, {"poll": {"question": "Was mitbringen?", "options": ["Chips", "Cola"], "multi": True}})
    check("multiple choice", call("POST", f"/channels/{general}/messages/{pm2['id']}/poll", c, {"option_ids": [x["id"] for x in pm2["poll"]["options"]]})[0] == 200)
    check("withdraw vote", call("POST", f"/channels/{general}/messages/{pm2['id']}/poll", c, {"option_ids": []})[0] == 200)
    e, eid = user("emil")
    f, fid = user("finn")
    check("friend request by name", call("POST", "/friends", d, {"username": "emil" + S})[0] == 200)
    check("duplicate request rejected", call("POST", "/friends", d, {"username": "emil" + S})[0] == 400)
    check("unknown user", call("POST", "/friends", d, {"username": "gibtsnicht" + S})[0] == 400)
    check("pending lists", [u["id"] for u in call("GET", "/friends", e)[1]["incoming"]] == [did] and [u["id"] for u in call("GET", "/friends", d)[1]["outgoing"]] == [eid])
    check("no DM before accepting", call("POST", "/users/@me/channels", d, {"user_id": eid})[0] == 403)
    check("accept", call("PUT", f"/friends/{did}", e)[0] == 200 and [u["id"] for u in call("GET", "/friends", d)[1]["friends"]] == [eid])
    check("cannot accept twice", call("PUT", f"/friends/{did}", e)[0] == 404)
    s, fdm = call("POST", "/users/@me/channels", d, {"user_id": eid})
    check("friends can DM without shared server", s == 200 and fdm["kind"] == "dm")
    call("POST", "/friends", f, {"user_id": did})
    check("mutual request accepts", call("POST", "/friends", d, {"user_id": fid})[0] == 200 and len(call("GET", "/friends", d)[1]["friends"]) == 2)
    check("group needs friends", call("POST", "/users/@me/groups", e, {"user_ids": [did, fid]})[0] == 400)
    s, grp = call("POST", "/users/@me/groups", d, {"user_ids": [eid], "name": "Runde"})
    check("create group", s == 200 and grp["kind"] == "group" and grp["owner_id"] == did and len(grp["recipients"]) == 2, grp)
    check("DM not confused with group", call("POST", "/users/@me/channels", d, {"user_id": eid})[1]["id"] == fdm["id"])
    check("add to group", call("PUT", f"/channels/{grp['id']}/recipients/{fid}", d)[0] == 200)
    check("only friends can be added", call("PUT", f"/channels/{grp['id']}/recipients/{aid}", d)[0] == 400)
    we, _ = await gateway(e)
    check("group message", call("POST", f"/channels/{grp['id']}/messages", f, {"content": "hallo gruppe"})[0] == 200)
    check("group message delivered", await wait_for(we, "MESSAGE_CREATE", match=lambda x: x["content"] == "hallo gruppe") is not None)
    check("outsider cannot read group", call("GET", f"/channels/{grp['id']}/messages", a)[0] == 404)
    check("rename group", call("PATCH", f"/channels/{grp['id']}/group", d, {"name": "Stammtisch"})[1]["name"] == "Stammtisch")
    gj = call("POST", "/users/@me/channels", d, {"user_id": eid})[1]
    check("roles in group", grp["roles"][did] == 2 and grp["roles"][eid] == 0 and grp["settings"] == {"invite": 0, "edit": 1, "kick": 1, "delete": 1}, grp)
    check("member cannot rename by default", call("PATCH", f"/channels/{grp['id']}/group", f, {"name": "x"})[0] == 403)
    check("member cannot remove others", call("DELETE", f"/channels/{grp['id']}/recipients/{fid}", e)[0] == 403)
    check("member cannot change roles", call("PUT", f"/channels/{grp['id']}/recipients/{eid}/role", e, {"role": 2})[0] == 403)
    check("admin makes moderator", call("PUT", f"/channels/{grp['id']}/recipients/{eid}/role", d, {"role": 1})[1]["roles"][eid] == 1)
    gm = call("POST", f"/channels/{grp['id']}/messages", f, {"content": "bitte löschen"})[1]
    gm = call("POST", f"/channels/{grp['id']}/messages", d, {"content": "vom admin"})[1]
    check("moderator deletes others' messages", call("DELETE", f"/channels/{grp['id']}/messages/{gm['id']}", e)[0] == 200)
    gm = call("POST", f"/channels/{grp['id']}/messages", d, {"content": "bleibt"})[1]
    check("member cannot delete others", call("DELETE", f"/channels/{grp['id']}/messages/{gm['id']}", f)[0] == 403)
    check("moderator cannot remove admin", call("DELETE", f"/channels/{grp['id']}/recipients/{did}", e)[0] == 403)
    check("moderator cannot change settings", call("PATCH", f"/channels/{grp['id']}/group", e, {"settings": {"kick": 0}})[0] == 403)
    check("admin tightens settings", call("PATCH", f"/channels/{grp['id']}/group", d, {"settings": {"invite": 2, "kick": 2}})[1]["settings"]["invite"] == 2)
    check("moderator can no longer kick", call("DELETE", f"/channels/{grp['id']}/recipients/{fid}", e)[0] == 403)
    r = requests.post(f"{BASE}/api/channels/{grp['id']}/group/icon", headers={"Authorization": f"Bearer {e}"}, files={"file": ("g.png", png, "image/png")})
    check("moderator sets group picture", r.status_code == 200 and r.json()["icon"].endswith(".png"), r.text)
    check("last admin cannot demote himself", call("PUT", f"/channels/{grp['id']}/recipients/{did}/role", d, {"role": 0})[0] == 400)
    check("owner removes member", call("DELETE", f"/channels/{grp['id']}/recipients/{fid}", d)[0] == 200 and call("GET", f"/channels/{grp['id']}/messages", f)[0] == 404)
    check("block", call("PUT", f"/blocks/{fid}", d)[0] == 200 and [u["id"] for u in call("GET", "/friends", d)[1]["blocked"]] == [fid] and len(call("GET", "/friends", f)[1]["friends"]) == 0)
    check("blocked user cannot send request", call("POST", "/friends", f, {"user_id": did})[0] == 400)
    check("blocked user cannot open DM", call("POST", "/users/@me/channels", f, {"user_id": did})[0] == 403)
    check("unblock", call("DELETE", f"/blocks/{fid}", d)[0] == 200 and call("GET", "/friends", d)[1]["blocked"] == [])
    call("PUT", f"/blocks/{eid}", d)
    check("no messages in DM while blocked", call("POST", f"/channels/{gj['id']}/messages", e, {"content": "hallo?"})[0] == 400 and call("POST", f"/channels/{gj['id']}/messages", d, {"content": "hallo?"})[0] == 400)
    call("DELETE", f"/blocks/{eid}", d)
    check("DM works again after unblock", call("POST", f"/channels/{gj['id']}/messages", e, {"content": "hallo"})[0] == 200)
    s, v = call("POST", f"/channels/{grp['id']}/voice", d)
    if s != 503:
        check("group call token", s == 200 and "token" in v)
        check("call rings others", await wait_for(we, "CALL_RING", match=lambda x: x["channel_id"] == grp["id"] and x["user_id"] == did) is not None)
        check("DM call", call("POST", f"/channels/{fdm['id']}/voice", e)[0] == 200)
        check("outsider cannot join call", call("POST", f"/channels/{grp['id']}/voice", a)[0] == 404)
        call("POST", f"/channels/{grp['id']}/voice", e)
        check("cannot watch without a stream", call("POST", "/voice/watch", d, {"user_id": eid, "on": True})[0] == 400)
        call("POST", "/voice/stream", e, {"on": True})
        check("watch stream", call("POST", "/voice/watch", d, {"user_id": eid, "on": True})[0] == 200)
        check("watchers announced", await wait_for(we, "VOICE_WATCHERS", match=lambda x: x["room"] == grp["id"] and x["watchers"] == [[did, eid]]) is not None)
        check("watchers given on join", call("POST", f"/channels/{grp['id']}/voice", e)[1]["watchers"] == [[did, eid]])
        call("POST", "/voice/stream", e, {"on": False})
        check("stream end clears watchers", await wait_for(we, "VOICE_WATCHERS", match=lambda x: x["watchers"] == []) is not None)
        call("DELETE", "/voice", d); call("DELETE", "/voice", e)
    check("owner leaves, ownership moves", call("DELETE", f"/channels/{grp['id']}/recipients/{did}", d)[0] == 200 and (await wait_for(we, "DM_UPDATE", match=lambda x: x["owner_id"] == eid and x["roles"][eid] == 2)) is not None)
    check("last member leaves, group is gone", call("DELETE", f"/channels/{grp['id']}/recipients/{eid}", e)[0] == 200 and call("GET", f"/channels/{grp['id']}/messages", e)[0] == 404)
    check("blocking ended the friendship", len(call("GET", "/friends", e)[1]["friends"]) == 0)
    await we.close()

    codes = [call("POST", f"/channels/{general}/messages", c, {"content": f"flut {i}"})[0] for i in range(12)]
    check("message rate limit", 429 in codes and codes[0] == 200, codes)

    check("ben bans cora? no permission", call("PUT", f"/guilds/{gid}/bans/{cid_user}", b, {})[0] == 403)
    check("owner bans cora", call("PUT", f"/guilds/{gid}/bans/{cid_user}", a, {"reason": "Test"})[0] == 200)
    check("cora gets GUILD_DELETE", await wait_for(wc, "GUILD_DELETE") is not None)
    check("banned cannot rejoin", call("POST", f"/invites/{inv['code']}", c)[0] == 400)
    check("ban list", call("GET", f"/guilds/{gid}/bans", a)[1][0]["reason"] == "Test")
    check("unban", call("DELETE", f"/guilds/{gid}/bans/{cid_user}", a)[0] == 200)
    check("rejoin works", call("POST", f"/invites/{inv['code']}", c)[0] == 200)

    check("owner cannot leave", call("DELETE", f"/guilds/{gid}/members/{aid}", a)[0] == 400)
    check("ben leaves", call("DELETE", f"/guilds/{gid}/members/{bid}", b)[0] == 200)
    check("ben cannot delete guild", call("DELETE", f"/guilds/{gid}", b)[0] == 404)
    check("logout", call("POST", "/auth/logout", b)[0] == 200 and call("GET", "/users/@me", b)[0] == 401)
    check("owner deletes guild", call("DELETE", f"/guilds/{gid}", a)[0] == 200)
    check("anna gets GUILD_DELETE", await wait_for(wa, "GUILD_DELETE") is not None)
    for w in (wa, wb, wc):
        await w.close()
    print("\n%d failed" % fails)
    sys.exit(1 if fails else 0)

asyncio.run(main())
