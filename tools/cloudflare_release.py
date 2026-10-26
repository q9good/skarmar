"""Release code/assets to the EXISTING Worker; never provision resources or bootstrap.

Only the sanitized receipt is suitable for a public Actions artifact. Credentials
are supplied by environment bindings; private config/logs stay in ignored paths.
"""

import argparse
import copy
import json
import os
from pathlib import Path
import re
import secrets
import subprocess
import sys
import time
import tomllib
import urllib.error
import urllib.parse
import urllib.request


ROOT = Path(__file__).resolve().parents[1]
CONFIG = ROOT / "deploy/cloud-worker/wrangler.toml"
OUTPUT = ROOT / ".local/cloudflare-release"
WORKER = "skarma-web"
ORIGIN = "https://skarma.vip"
SHA = re.compile(r"[0-9a-f]{40}")
UUID = re.compile(r"[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}")
PRD_VARS = {"SKARMA_MODEL": "prd_v1", "SKARMA_PRD_BASELINE": "empty_preserve_legacy_v1"}


class ReleaseError(Exception):
    """A deliberately non-sensitive error that may be printed in public logs."""


def require(condition, message):
    if not condition:
        raise ReleaseError(message)


def validate_config(config):
    require(config.get("name") == WORKER, "Configuration must target the existing skarma-web.")
    require(re.fullmatch(r"[0-9a-f]{32}", config.get("account_id", "")) is not None,
            "An explicit existing account_id is required in the private config.")
    require(config.get("main") == "../../crates/worker/build/index.js", "Unexpected Worker entrypoint.")
    require(config.get("build") == {"command": "worker-build --release", "cwd": "crates/worker"},
            "Unexpected build command; arbitrary private-config commands are not allowed.")
    require(config.get("assets") == {
        "directory": "../../apps/client/dist", "not_found_handling": "single-page-application",
        "run_worker_first": ["/api", "/api/*"]}, "Unexpected asset routing configuration.")
    require(config.get("durable_objects") == {
        "bindings": [{"name": "SKARMA_DB", "class_name": "SkarmaSpace"}]},
        "Only the existing local SkarmaSpace binding is supported.")
    require(config.get("migrations") == [{"tag": "v1", "new_sqlite_classes": ["SkarmaSpace"]}],
            "Namespace migrations require a separate reviewed procedure.")
    require(config.get("routes") == [{"pattern": "skarma.vip", "custom_domain": True}],
            "The private config must retain the skarma.vip Custom Domain.")
    require(config.get("workers_dev") is True and config.get("preview_urls") is False,
            "Retain the original workers.dev entry and disabled preview URLs.")
    require(re.fullmatch(r"\d{4}-\d{2}-\d{2}", config.get("compatibility_date", "")) is not None,
            "An explicit compatibility date is required.")
    variables = config.get("vars", {})
    expected = {"SKARMA_STORAGE": "durable_object", "SKARMA_DEMO": "0", "SKARMA_SPACE": "default",
                "SKARMA_AUTH": "required", "SKARMA_PRODUCTION": "1",
                "SKARMA_ADDITIONAL_PUBLIC_URLS": ORIGIN}
    require(all(variables.get(k) == v for k, v in expected.items()),
            "Storage, space, authentication or exact additional origin differs from production.")
    primary = variables.get("SKARMA_PUBLIC_URL", "")
    require(re.fullmatch(r"https://skarma-web\.[a-z0-9-]+\.workers\.dev", primary) is not None
            and "ACCOUNT_SUBDOMAIN" not in primary, "The original exact public URL is required.")
    require(all(isinstance(v, str) for v in variables.values()), "Only plain-text application vars are supported.")
    require(not any(re.search(r"password|secret|token|cookie", k, re.I) for k in variables),
            "Credentials/bootstrap passwords must not appear in config vars.")
    model_vars = {k: variables[k] for k in PRD_VARS if k in variables}
    require(not model_vars or model_vars == PRD_VARS,
            "Only the reviewed empty PRD baseline is supported by this release flow.")
    allowed = {"name", "account_id", "main", "compatibility_date", "compatibility_flags", "build",
               "assets", "vars", "durable_objects", "migrations", "routes", "workers_dev", "preview_urls",
               "keep_vars", "observability", "limits", "placement", "send_metrics"}
    require(set(config) <= allowed, "Unsupported config sections require explicit review.")
    return config


def invariant(version):
    resources = version.get("resources", {})
    bindings = resources.get("bindings", [])
    require(isinstance(bindings, list) and bindings, "Version binding metadata is unavailable.")
    do = [b for b in bindings if b.get("type") == "durable_object_namespace"]
    require(len(do) == 1 and do[0].get("name") == "SKARMA_DB"
            and do[0].get("class_name") == "SkarmaSpace"
            and do[0].get("namespace_id")
            and do[0].get("script_name", WORKER) in (None, WORKER),
            "The existing durable object identity could not be verified.")
    require(not any(b.get("name") == "SKARMA_INITIAL_ADMIN_PASSWORD" for b in bindings),
            "Unexpected bootstrap secret: investigate; never initialize another administrator.")
    supported = {"plain_text", "secret_text", "durable_object_namespace", "assets"}
    require(all(b.get("type") in supported for b in bindings), "Unsupported production binding type.")
    # Asset content and code change, but all storage identities/vars/secret names must remain.
    preserved = [b for b in bindings if b.get("type") != "assets"]
    return {"bindings": sorted(preserved, key=lambda b: b["name"]),
            "runtime": resources.get("script_runtime", {})}


def validate_live(config, version):
    preserved = invariant(version)
    variables = {b["name"]: b.get("text") for b in preserved["bindings"] if b["type"] == "plain_text"}
    # The original private Secret can remain unchanged after the explicitly
    # reviewed cutover. Accept ONLY these exact added model vars; all other
    # live settings must still equal the private original configuration.
    expected = config["vars"]
    matches_prd_overlay = not any(k in expected for k in PRD_VARS) and variables == {**expected, **PRD_VARS}
    require(variables == expected or matches_prd_overlay,
            "Private configuration vars differ from the live version; refresh it safely.")
    runtime = preserved["runtime"]
    require(config.get("compatibility_date") == runtime.get("compatibility_date"),
            "Compatibility date must match the live version.")
    require(sorted(config.get("compatibility_flags", [])) == sorted(runtime.get("compatibility_flags", [])),
            "Compatibility flags must match the live version.")
    return preserved


class Cloudflare:
    def __init__(self, account_id):
        self.root = f"https://api.cloudflare.com/client/v4/accounts/{account_id}/workers"

    def request(self, suffix, data=None):
        operation = ("GET " if data is None else "POST ") + UUID.sub("<version>", suffix)
        request = urllib.request.Request(self.root + suffix, headers={
            "Authorization": "Bearer " + os.environ["CLOUDFLARE_API_TOKEN"],
            "Accept": "application/json", "Content-Type": "application/json"},
            data=json.dumps(data).encode() if data is not None else None)
        try:
            with urllib.request.urlopen(request, timeout=45) as response:
                body = json.load(response)
        except urllib.error.HTTPError as error:
            raise ReleaseError(f"Cloudflare {operation} failed (HTTP {error.code}); check token scope and API permissions.") from None
        except (urllib.error.URLError, TimeoutError, ValueError):
            raise ReleaseError("Cloudflare API request failed; no deployment is assumed successful.") from None
        require(body.get("success") is True, "Cloudflare did not confirm the API request.")
        return body["result"]

    def get(self, suffix):
        return self.request(suffix)

    def promote(self, version_id, source_sha):
        # A version deployment only; unlike the CLI it never also PATCHes script-settings.
        return self.request(f"/scripts/{WORKER}/deployments", {
            "strategy": "percentage", "versions": [{"version_id": version_id, "percentage": 100}],
            "annotations": {"workers/message": f"GitHub {source_sha}"}})

    def current(self):
        deployments = self.get(f"/scripts/{WORKER}/deployments").get("deployments", [])
        require(bool(deployments), "Existing deployment not found; refusing to create an instance.")
        latest = deployments[0]
        versions = latest.get("versions", [])
        require(len(versions) == 1 and versions[0].get("percentage") == 100,
                "An existing split deployment requires separate review.")
        version_id = versions[0]["version_id"]
        require(UUID.fullmatch(version_id) is not None, "Invalid deployment version metadata.")
        return latest["id"], version_id, self.get(f"/scripts/{WORKER}/versions/{version_id}")

    def routing(self):
        result = self.get("/domains?" + urllib.parse.urlencode({"service": WORKER}))
        require(isinstance(result, list), "Custom Domain metadata unavailable.")
        domains = [d for d in result if d.get("service") == WORKER]
        require(len(domains) == 1 and domains[0].get("hostname") == "skarma.vip"
                and domains[0].get("environment") == "production", "Existing Custom Domain differs from expected.")
        subdomain = self.get(f"/scripts/{WORKER}/subdomain")
        require(subdomain.get("enabled") is True and subdomain.get("previews_enabled") is False,
                "Existing workers.dev/preview settings differ from expected.")
        settings = self.get(f"/scripts/{WORKER}/script-settings")
        return {"domains": domains, "subdomain": subdomain, "settings": settings}


def wrangler(args, label):
    private = OUTPUT / "private"
    private.mkdir(parents=True, exist_ok=True, mode=0o700)
    output_file = private / (label + ".ndjson")
    output_file.unlink(missing_ok=True)
    env = {**os.environ, "WRANGLER_SEND_METRICS": "false", "CI": "true",
           "WRANGLER_OUTPUT_FILE_PATH": str(output_file)}
    result = subprocess.run([str(ROOT / "node_modules/.bin/wrangler"), *args,
                             "--config", str(CONFIG)], cwd=ROOT, env=env,
                            stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    # Wrangler prints binding values; never echo or upload its raw output in this public repo.
    (private / (label + ".log")).write_text(result.stdout)
    codes = sorted(set(re.findall(r"\[code: (\d+)\]", result.stdout)))
    require(result.returncode == 0,
            f"Wrangler {label} failed (API codes: {','.join(codes) or 'unavailable'}); raw log was not published.")
    return [json.loads(line) for line in output_file.read_text().splitlines() if line.strip()] if output_file.exists() else []


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args):
        return None


def public_request(path, headers=None, method="GET"):
    request = urllib.request.Request(ORIGIN + path,
                                     headers={"User-Agent": "SkarmaReleaseCheck/1.0", **(headers or {})}, method=method,
                                     data=b"{}" if method == "POST" else None)
    try:
        with urllib.request.build_opener(NoRedirect).open(request, timeout=30) as response:
            return response.status, response.read()
    except urllib.error.HTTPError as error:
        return error.code, error.read()
    except (urllib.error.URLError, TimeoutError):
        raise ReleaseError("Public HTTPS request failed; production verification remains incomplete.") from None


def normalize_homepage(body):
    # Cloudflare Web Analytics adds this tag plus a newline after serving the asset.
    # Remove ONLY that exact external script, never application or unknown scripts.
    count = 0

    def remove_beacon(match):
        nonlocal count
        src = re.search(rb'\ssrc\s*=\s*["\x27]([^"\x27]+)["\x27]', match[0], re.I)
        if src and re.fullmatch(rb"https://static\.cloudflareinsights\.com/beacon\.min\.js(?:/[A-Za-z0-9]+)?", src[1]):
            count += 1
            return b""
        return match[0]

    normalized = re.sub(rb"<script\b[^>]*>\s*</script>(?:\r?\n)?", remove_beacon, body, flags=re.I)
    require(count <= 1, "Unexpected duplicate Cloudflare Analytics injection.")
    return normalized, count


def smoke(initialize_prd=False):
    status, body = public_request("/")
    codes = re.findall(rb"(?:error code[: ]+|Error\s+)(\d{4})", body, re.I)
    require(status == 200, f"Public homepage returned HTTP {status} (edge codes: "
            + ",".join(code.decode() for code in codes) + "); public verification remains incomplete.")
    index = (ROOT / "apps/client/dist/index.html").read_bytes()
    normalized, beacon_count = normalize_homepage(body)
    require(normalized == index, "Public homepage differs from this run's web build.")
    scripts = re.findall(rb'<script\b[^>]*\bsrc="([^"]+)"', index)
    require(bool(scripts), "No local JavaScript entrypoint found.")
    for raw in scripts:
        path = raw.decode()
        require(path.startswith("/") and not path.startswith("//") and ".." not in path,
                "Unexpected external JavaScript entrypoint.")
        status, body = public_request(path)
        require(status == 200 and body == (ROOT / "apps/client/dist" / path.lstrip("/")).read_bytes(),
                "Public JavaScript does not match this run's assets.")
    status, body = public_request("/api/health")
    require(status == 200 and json.loads(body).get("status") == "ok", "Health verification failed.")
    for path, expected in [("/api/auth/me", 401), ("/api/state", 401), ("/api/nonexistent-release-check", 404)]:
        require(public_request(path)[0] == expected, "Anonymous API verification failed.")
    for origin, marker, expected in [(ORIGIN, True, 200), ("https://invalid.example", True, 403), (ORIGIN, False, 403)]:
        headers = {"Origin": origin, "Content-Type": "application/json"}
        if marker:
            headers["x-skarma-request"] = "1"
        require(public_request("/api/auth/logout", headers, "POST")[0] == expected,
                "Exact-origin verification failed.")
    checks = {"homepage": "matched" if not beacon_count else "matched_except_known_cf_analytics",
            "javascript": "matched", "cf_analytics_tags": beacon_count,
            "health": 200, "anonymous": "passed", "origins": "passed"}
    if initialize_prd:
        # An unissued random session's logout enters the DO transaction without
        # logging in, reading business data, or modifying anyone's session.
        # Anonymous logout alone never calls the repository and cannot prove DB startup.
        status, body = public_request("/api/auth/logout", {
            "Origin": ORIGIN, "Content-Type": "application/json", "x-skarma-request": "1",
            "Cookie": "__Host-skarma_session=" + secrets.token_hex(32)}, "POST")
        require(status == 200 and json.loads(body).get("ok") is True,
                "PRD database startup did not complete; do not claim the new model is usable.")
        checks["prd_database"] = "startup_completed"
    return checks


def run_release(config, source_sha, api, receipt, activate_prd_empty=False):
    deployment_id, previous_id, previous = api.current()
    preserved = validate_live(config, previous)
    expected = copy.deepcopy(preserved)
    extra_args = []
    live_vars = {b["name"]: b.get("text") for b in preserved["bindings"] if b["type"] == "plain_text"}
    if activate_prd_empty:
        require(not any(k in live_vars for k in PRD_VARS),
                "PRD model is already configured; use an ordinary release, never reinitialize.")
        for key, value in PRD_VARS.items():
            expected["bindings"].append({"type": "plain_text", "name": key, "text": value})
            extra_args.extend(["--var", key + ":" + value])
        expected["bindings"].sort(key=lambda b: b["name"])
    prd = all({b["name"]: b.get("text") for b in expected["bindings"]}.get(k) == v for k, v in PRD_VARS.items())
    receipt["model"] = "prd_v1" if prd else "legacy"
    routing = api.routing()
    receipt.update(previous_version=previous_id, state="preflight_passed")
    wrangler(["versions", "upload", "--dry-run", "--keep-vars", *extra_args], "dry-run")
    # Recheck after a slow build; concurrent Dashboard/Mac releases must not be overwritten.
    require(api.current()[0] == deployment_id and api.routing() == routing,
            "Production changed during preparation; refusing to overwrite another release.")
    events = wrangler(["versions", "upload", "--keep-vars", "--message", f"GitHub {source_sha}", *extra_args], "upload")
    uploads = [e for e in events if e.get("type") == "version-upload"]
    require(len(uploads) == 1 and UUID.fullmatch(uploads[0].get("version_id", "")) is not None,
            "Upload version ID was not confirmed; no traffic switch attempted.")
    new_id = uploads[0]["version_id"]
    receipt.update(uploaded_version=new_id, state="uploaded_not_promoted")
    candidate = api.get(f"/scripts/{WORKER}/versions/{new_id}")
    require(invariant(candidate) == expected,
            "Uploaded version changes storage, variables or runtime settings; no traffic switch attempted.")
    require(api.current()[0] == deployment_id and api.routing() == routing,
            "Production changed before promotion; uploaded version remains inactive.")
    receipt["state"] = "promotion_attempted"
    api.promote(new_id, source_sha)
    _, active_id, active = api.current()
    require(active_id == new_id and invariant(active) == expected and api.routing() == routing,
            "Post-deploy metadata differs; investigate before any further release (no automatic rollback).")
    receipt.update(deployed_version=new_id, state="deployed_public_checks_pending")
    last_error = None
    for attempt in range(3):
        try:
            receipt["checks"] = smoke(initialize_prd=prd)
            receipt["state"] = "deployed_and_verified"
            return
        except ReleaseError as error:
            last_error = error
            if attempt < 2:
                time.sleep(5)
    raise last_error


def verify_existing(config, expected_version, api, receipt):
    """Authenticate and verify an active version without any upload or traffic mutation."""
    require(UUID.fullmatch(expected_version) is not None, "Require an exact existing version UUID.")
    deployment_id, active_id, active = api.current()
    require(active_id == expected_version, "Another version is active; refusing to verify the wrong release.")
    preserved = validate_live(config, active)
    routing = api.routing()
    receipt.update(deployed_version=active_id, state="deployed_public_checks_pending")
    message = active.get("annotations", {}).get("workers/message", "")
    source = re.fullmatch(r"GitHub ([0-9a-f]{40})", message)
    if source:
        receipt["deployed_source_sha"] = source[1]
    live_vars = {b["name"]: b.get("text") for b in preserved["bindings"] if b["type"] == "plain_text"}
    prd = all(live_vars.get(k) == v for k, v in PRD_VARS.items())
    receipt["model"] = "prd_v1" if prd else "legacy"
    receipt["checks"] = smoke(initialize_prd=prd)
    final_deployment, final_id, final = api.current()
    require(final_deployment == deployment_id and final_id == active_id
            and invariant(final) == preserved and api.routing() == routing,
            "Production changed during verification; verification is not complete.")
    receipt["state"] = "verified_existing_version"


def verification_version_from_event():
    event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text())
    message = event.get("head_commit", {}).get("message", "")
    matches = re.findall(r"\[verify-cloudflare:([0-9a-f-]{36})\]", message)
    require(len(matches) == 1 and UUID.fullmatch(matches[0]) is not None,
            "Require one explicit verify-cloudflare version marker.")
    return matches[0]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-sha", required=True)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--deploy", action="store_true", help="Explicitly upload and promote the existing Worker.")
    mode.add_argument("--verify-version", help="Only verify this active version; never upload or promote.")
    mode.add_argument("--verify-from-event", action="store_true", help="Read an explicit verify version marker from the GitHub event.")
    parser.add_argument("--activate-prd-empty", action="store_true", help="Explicit one-time empty PRD domain; preserve original auth and legacy tables.")
    args = parser.parse_args()
    receipt = {"worker": WORKER, "origin": ORIGIN, "source_sha": args.source_sha, "state": "not_deployed"}
    os.umask(0o077)
    OUTPUT.mkdir(parents=True, exist_ok=True)
    wrote_config = False
    try:
        require(SHA.fullmatch(args.source_sha) is not None, "Require an exact 40-character source SHA.")
        actual_sha = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
        require(actual_sha == args.source_sha, "Checkout does not match the requested source SHA.")
        require(not subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT, text=True).strip(),
                "Release requires a clean committed checkout.")
        require(not CONFIG.exists(), "Refusing to overwrite an existing private configuration.")
        raw = os.environ.get("SKARMA_WRANGLER_TOML", "")
        require(bool(raw.strip()), "Missing SKARMA_WRANGLER_TOML secret: use the existing private production config.")
        try:
            config = validate_config(tomllib.loads(raw))
        except (tomllib.TOMLDecodeError, TypeError, AttributeError):
            raise ReleaseError("Invalid private production config; its contents were not logged.") from None
        CONFIG.write_text(raw)
        wrote_config = True
        expected_version = verification_version_from_event() if args.verify_from_event else args.verify_version
        require(not args.activate_prd_empty or args.deploy,
                "PRD cutover requires explicit deploy mode; verification cannot change the model.")
        if not args.deploy and not expected_version:
            receipt["state"] = "config_validated_only"
            return
        require(bool(os.environ.get("CLOUDFLARE_API_TOKEN")), "CLOUDFLARE_API_TOKEN is not bound.")
        api = Cloudflare(config["account_id"])
        receipt["operation"] = "verify_existing" if expected_version else "deploy"
        if expected_version:
            verify_existing(config, expected_version, api, receipt)
        else:
            run_release(config, args.source_sha, api, receipt, activate_prd_empty=args.activate_prd_empty)
    except (ReleaseError, OSError, ValueError, KeyError) as error:
        # Unexpected exceptions/JSON/OS errors can contain private paths or API values.
        receipt["error"] = str(error) if isinstance(error, ReleaseError) else "Release failed; inspect private runner diagnostics."
        print(receipt["error"], file=sys.stderr)
        return 1
    finally:
        if wrote_config:
            CONFIG.unlink(missing_ok=True)
        (OUTPUT / "receipt.json").write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + "\n")
        print(json.dumps(receipt, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    sys.exit(main())
