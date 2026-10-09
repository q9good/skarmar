"""Check the local synthetic-data Worker; does not contact Cloudflare."""
import json
import urllib.error
import urllib.request

BASE = "http://127.0.0.1:8787"


def call(path, status=200):
    try:
        with urllib.request.urlopen(BASE + path, timeout=10) as response:
            code, body = response.status, response.read().decode()
    except urllib.error.HTTPError as error:
        code, body = error.code, error.read().decode()
    assert code == status, (path, code, body)
    print(path, code, body)
    return body


assert json.loads(call("/api/health")) == {"language": "Rust", "framework": "Axum 0.8"}
call("/probe/reset")
call("/probe/fail", 409)
assert json.loads(call("/probe/count")) == [{"count": 0}]
call("/probe/commit")
assert json.loads(call("/probe/count")) == [{"count": 2}]
call("/probe/reset")
print("PASS: Rust Axum HTTP and Rust DO SQLite transaction rollback/commit")
