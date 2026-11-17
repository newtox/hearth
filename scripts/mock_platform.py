import json, sys
from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import urlparse, parse_qs
STATE={"playing":True,"track":"One More Time","refreshes":0,"token":"acc1","subs":["42"],"short":False,"mrefresh":0,"subcalls":0,"scope":""}
class H(BaseHTTPRequestHandler):
    def log_message(self,*a): pass
    def out(self,code,body=None,headers={}):
        self.send_response(code)
        for k,v in headers.items(): self.send_header(k,v)
        data=json.dumps(body).encode() if body is not None else b""
        self.send_header("Content-Type","application/json"); self.send_header("Content-Length",str(len(data))); self.end_headers(); self.wfile.write(data)
    def do_GET(self):
        u=urlparse(self.path); q=parse_qs(u.query, keep_blank_values=True)
        if u.path=="/authorize":
            assert q["scope"]==["user-read-currently-playing"] and q["client_id"]==["cid"]
            return self.out(302,None,{"Location":q["redirect_uri"][0]+"?code=thecode&state="+q["state"][0]})
        if u.path=="/v1/me/player/currently-playing":
            if self.headers.get("Authorization")!="Bearer "+STATE["token"]: return self.out(401,{})
            if not STATE["playing"]: return self.out(204)
            return self.out(200,{"is_playing":True,"item":{"name":STATE["track"],"artists":[{"name":"Daft Punk"}]}})
        if u.path=="/oauth/subs":
            STATE["subcalls"]+=1
            if self.headers.get("Authorization")!="Bearer macc": return self.out(401,{})
            assert q["broadcaster_id"]==["42"]
            return self.out(200,{"data":[{"user_id":x} for x in STATE["subs"]],"pagination":{}})
        if u.path=="/oauth/authorize":
            assert q["client_id"]==["mock-id"] and "redirect_uri" in q
            STATE["scope"]=q["scope"][0]
            return self.out(302,None,{"Location":q["redirect_uri"][0]+"?code=c0de&state="+q["state"][0]})
        if u.path=="/oauth/user":
            if self.headers.get("Authorization")!="Bearer macc": return self.out(401,{})
            return self.out(200,{"id":42,"name":"mockuser","url":"https://example.com/mockuser"})
        if u.path=="/v1/me":
            return self.out(200,{"id":"sp1","display_name":"Spoti","external_urls":{"spotify":"https://open.spotify.com/user/sp1"}})
        if u.path=="/ctl":
            if "playing" in q: STATE["playing"]=q["playing"][0]=="1"
            if "track" in q: STATE["track"]=q["track"][0]
            if "subs" in q: STATE["subs"]=[x for x in q["subs"][0].split(",") if x]
            if "short" in q: STATE["short"]=q["short"][0]=="1"
            return self.out(200,STATE)
        self.out(404,{})
    def do_POST(self):
        body=parse_qs(self.rfile.read(int(self.headers.get("Content-Length",0))).decode())
        if self.path=="/oauth/token":
            assert body["client_id"]==["mock-id"] and body["client_secret"]==["mock-secret"]
            if body["grant_type"]==["refresh_token"]:
                assert body["refresh_token"]==["mref"]; STATE["mrefresh"]+=1
                return self.out(200,{"access_token":"macc","expires_in":3600})
            assert body["code"]==["c0de"]
            if STATE["short"]: return self.out(200,{"access_token":"macc","refresh_token":"mref","expires_in":30})
            return self.out(200,{"access_token":"macc"})
        if self.path=="/api/token":
            import base64
            assert self.headers["Authorization"]=="Basic "+base64.b64encode(b"cid:sec").decode()
            if body["grant_type"]==["authorization_code"]:
                assert body["code"]==["thecode"]
                return self.out(200,{"access_token":"acc0","refresh_token":"ref","expires_in":30})
            assert body["refresh_token"]==["ref"]; STATE["refreshes"]+=1
            return self.out(200,{"access_token":"acc1","expires_in":3600})
        self.out(404,{})
HTTPServer(("127.0.0.1",9099),H).serve_forever()
