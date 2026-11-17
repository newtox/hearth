import requests, time
import sys
B=(sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:8080").rstrip("/")+"/api"; M="http://127.0.0.1:9099"
S=str(int(time.time()*1000))[-7:]
fails=0
def ok(n,c,i=""):
    global fails
    if not c: fails+=1
    print(("ok   " if c else "FAIL ")+n, "" if c else i)
def user(n):
    t=requests.post(B+"/auth/register",json={"username":n+S,"email":n+S+"@example.com","password":"geheim1234"}).json()["token"]
    return {"Authorization":"Bearer "+t}
A,C,D=user("owner"),user("fan"),user("other")
g=requests.post(B+"/guilds",headers=A,json={"name":"Streamhaus"}).json()
gid=g["id"]
inv=requests.post(B+f"/guilds/{gid}/invites",headers=A,json={}).json()["code"]
for h in (C,D): requests.post(B+f"/invites/{inv}",headers=h)
snap=lambda: requests.get(B+f"/guilds/{gid}",headers=A).json()
ok("member cannot list integrations", requests.get(B+f"/guilds/{gid}/integrations",headers=C).status_code==403)
lst=requests.get(B+f"/guilds/{gid}/integrations",headers=A).json()
ok("mock available", lst=={"available":[{"id":"mock","name":"Mock"}],"linked":[]}, lst)
ok("unknown provider", requests.post(B+f"/guilds/{gid}/integrations/steam",headers=A).status_code==404)
ok("member cannot start", requests.post(B+f"/guilds/{gid}/integrations/mock",headers=C).status_code==403)
requests.get(M+"/ctl?short=1")
url=requests.post(B+f"/guilds/{gid}/integrations/mock",headers=A).json()["url"]
r=requests.get(url)
ok("callback returns to app", r.url.endswith("/?integration=ok"), r.url)
ok("asked for subscriber scope", requests.get(M+"/ctl").json()["scope"]=="subs")
lst=requests.get(B+f"/guilds/{gid}/integrations",headers=A).json()["linked"]
ok("integration listed", len(lst)==1 and lst[0]["account_name"]=="mockuser", lst)
iid=lst[0]["id"]; rid=lst[0]["role_id"]
role=next(r for r in snap()["roles"] if r["id"]==rid)
ok("managed role created", role["managed"]=="mock" and role["name"]=="Supporter" and role["permissions"]==0, role)
ok("role is not a connection", requests.get(B+"/users/@me",headers=A).json()["connections"]==[])
roles_of=lambda h: next(m for m in snap()["members"] if m["id"]==requests.get(B+"/users/@me",headers=h).json()["id"])["roles"]
ok("nobody has it before linking", rid not in roles_of(C))
requests.get(requests.get(B+"/connections/mock",headers=C).json()["url"])
time.sleep(1)
ok("fan gets role after connecting account", rid in roles_of(C))
ok("other member without account has no role", rid not in roles_of(D))
cid=requests.get(B+"/users/@me",headers=C).json()["id"]
requests.patch(B+f"/guilds/{gid}/members/{cid}",headers=A,json={"roles":[]})
ok("manual removal ignored", rid in roles_of(C))
did=requests.get(B+"/users/@me",headers=D).json()["id"]
requests.patch(B+f"/guilds/{gid}/members/{did}",headers=A,json={"roles":[rid]})
ok("manual grant ignored", rid not in roles_of(D))
ok("managed role cannot be deleted", requests.delete(B+f"/guilds/{gid}/roles/{rid}",headers=A).status_code==400)
ok("managed role can be renamed", requests.patch(B+f"/guilds/{gid}/roles/{rid}",headers=A,json={"name":"Subs"}).status_code==200)
requests.get(M+"/ctl?subs=")
ok("sync now", requests.post(B+f"/guilds/{gid}/integrations/{iid}/sync",headers=A).status_code==200)
ok("role removed when subscription ends", rid not in roles_of(C))
lst=requests.get(B+f"/guilds/{gid}/integrations",headers=A).json()["linked"][0]
ok("count and sync time", lst["members"]==0 and lst["synced_at"] and lst["error"] is None, lst)
requests.get(M+"/ctl?subs=42")
time.sleep(7)
ok("background sync gives role back", rid in roles_of(C))
ok("token was refreshed", requests.get(M+"/ctl").json()["mrefresh"]>=1)
requests.delete(B+"/connections/mock",headers=C); time.sleep(1)
ok("disconnecting the account removes role", rid not in roles_of(C))
requests.get(requests.get(B+"/connections/mock",headers=C).json()["url"]); time.sleep(1)
requests.delete(B+f"/guilds/{gid}/members/{cid}",headers=C)
requests.post(B+f"/invites/{inv}",headers=C); time.sleep(1)
ok("rejoining member gets role", rid in roles_of(C))
url=requests.post(B+f"/guilds/{gid}/integrations/mock",headers=A).json()["url"]; requests.get(url)
lst=requests.get(B+f"/guilds/{gid}/integrations",headers=A).json()["linked"]
ok("relinking keeps one integration and role", len(lst)==1 and lst[0]["role_id"]==rid and sum(1 for r in snap()["roles"] if r["managed"])==1)
ok("bad integration id", requests.post(B+f"/guilds/{gid}/integrations/nope/sync",headers=A).status_code==404)
ok("member cannot remove", requests.delete(B+f"/guilds/{gid}/integrations/{iid}",headers=C).status_code==403)
ok("remove integration", requests.delete(B+f"/guilds/{gid}/integrations/{iid}",headers=A).status_code==200)
ok("role gone", not any(r["id"]==rid for r in snap()["roles"]) and requests.get(B+f"/guilds/{gid}/integrations",headers=A).json()["linked"]==[])
audit=[a["action"] for a in requests.get(B+f"/guilds/{gid}/audit",headers=A).json()]
ok("audit log", "integration_create" in audit and "integration_delete" in audit, audit)
print(fails,"failed")
sys.exit(1 if fails else 0)
