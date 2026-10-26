"""Synthetic release contract: refusal before promotion is as important as success."""

import copy
import json
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch

import cloudflare_release as release


OLD = "11111111-1111-1111-1111-111111111111"
NEW = "22222222-2222-2222-2222-222222222222"
SOURCE = "a" * 40


def config():
    return {
        "name": "skarma-web", "account_id": "a" * 32,
        "main": "../../crates/worker/build/index.js", "compatibility_date": "2026-10-18",
        "workers_dev": True, "preview_urls": False,
        "build": {"command": "worker-build --release", "cwd": "crates/worker"},
        "assets": {"directory": "../../apps/client/dist", "not_found_handling": "single-page-application",
                   "run_worker_first": ["/api", "/api/*"]},
        "durable_objects": {"bindings": [{"name": "SKARMA_DB", "class_name": "SkarmaSpace"}]},
        "migrations": [{"tag": "v1", "new_sqlite_classes": ["SkarmaSpace"]}],
        "routes": [{"pattern": "skarma.vip", "custom_domain": True}],
        "vars": {"SKARMA_STORAGE": "durable_object", "SKARMA_DEMO": "0", "SKARMA_SPACE": "default",
                 "SKARMA_AUTH": "required", "SKARMA_PRODUCTION": "1",
                 "SKARMA_PUBLIC_URL": "https://skarma-web.synthetic.workers.dev",
                 "SKARMA_ADDITIONAL_PUBLIC_URLS": "https://skarma.vip"}}


def version(version_id=OLD):
    return {"id": version_id, "resources": {
        "bindings": [{"type": "plain_text", "name": k, "text": v} for k, v in config()["vars"].items()]
        + [{"type": "durable_object_namespace", "name": "SKARMA_DB", "class_name": "SkarmaSpace",
            "namespace_id": "synthetic-namespace", "script_name": "skarma-web"}],
        "script_runtime": {"compatibility_date": "2026-10-18", "compatibility_flags": []}}}


class FakeAPI:
    def __init__(self):
        self.active = OLD
        self.old = version()
        self.candidate = version(NEW)
        self.calls = 0
        self.change_at = None
        self.commands = []

    def current(self):
        self.calls += 1
        return ("concurrent" if self.calls == self.change_at else "deployment-" + self.active,
                self.active, copy.deepcopy(self.old if self.active == OLD else self.candidate))

    def routing(self):
        return {"domains": [{"hostname": "skarma.vip"}], "subdomain": {"enabled": True}, "settings": {}}

    def get(self, _):
        return copy.deepcopy(self.candidate)

    def promote(self, version_id, source_sha):
        assert version_id == NEW and source_sha == SOURCE
        self.commands.append("promote")
        self.active = version_id


class ConfigTests(unittest.TestCase):
    def test_valid_existing_config(self):
        release.validate_config(config())
        release.validate_live(config(), version())

    def test_wrong_worker_space_auth_and_placeholder_refused(self):
        mutations = [lambda c: c.update(name="new-worker"),
                     lambda c: c.pop("account_id"),
                     lambda c: c["vars"].update(SKARMA_SPACE="new-space"),
                     lambda c: c["vars"].update(SKARMA_AUTH="demo"),
                     lambda c: c["vars"].update(SKARMA_DEMO="1"),
                     lambda c: c["vars"].update(SKARMA_PUBLIC_URL="https://skarma-web.ACCOUNT_SUBDOMAIN.workers.dev"),
                     lambda c: c["vars"].update(SKARMA_ADDITIONAL_PUBLIC_URLS="*"),
                     lambda c: c["vars"].update(SKARMA_INITIAL_ADMIN_PASSWORD="never-a-real-password"),
                     lambda c: c["routes"].clear(),
                     lambda c: c["migrations"].append({"tag": "v2", "deleted_classes": ["SkarmaSpace"]}),
                     lambda c: c["build"].update(command="unreviewed-command"),
                     lambda c: c.update(unsafe={"bindings": []}),
                     lambda c: c["durable_objects"]["bindings"][0].update(script_name="other")]
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                value = config()
                mutate(value)
                with self.assertRaises(release.ReleaseError):
                    release.validate_config(value)

    def test_changed_live_vars_or_date_refused(self):
        for mutate in [lambda v: v["resources"]["bindings"][0].update(text="different"),
                       lambda v: v["resources"]["script_runtime"].update(compatibility_date="2026-10-01")]:
            value = version()
            mutate(value)
            with self.assertRaises(release.ReleaseError):
                release.validate_live(config(), value)

    def test_missing_namespace_or_bootstrap_refused(self):
        for mutate in [lambda v: v["resources"]["bindings"][-1].pop("namespace_id"),
                       lambda v: v["resources"]["bindings"].append({"type": "secret_text", "name": "SKARMA_INITIAL_ADMIN_PASSWORD"})]:
            value = version()
            mutate(value)
            with self.assertRaises(release.ReleaseError):
                release.invariant(value)


class PromotionTests(unittest.TestCase):
    def setUp(self):
        self.api = FakeAPI()
        self.commands = []
        self.api.commands = self.commands
        self.receipt = {}

    def wrangler(self, args, label):
        self.commands.append(label)
        if label == "upload":
            return [{"type": "version-upload", "version_id": NEW}]
        return []

    def run_release(self):
        with patch.object(release, "wrangler", side_effect=self.wrangler), \
                patch.object(release, "smoke", return_value={"synthetic": "passed"}), \
                patch.object(release.time, "sleep"):
            release.run_release(config(), SOURCE, self.api, self.receipt)

    def test_success_preserves_identity_and_reports_actual_version(self):
        self.run_release()
        self.assertEqual(self.commands, ["dry-run", "upload", "promote"])
        self.assertEqual(self.receipt["previous_version"], OLD)
        self.assertEqual(self.receipt["deployed_version"], NEW)
        self.assertEqual(self.receipt["state"], "deployed_and_verified")

    def test_dry_run_is_not_deployment(self):
        with patch.object(release, "wrangler", side_effect=release.ReleaseError("synthetic build failure")):
            with self.assertRaises(release.ReleaseError):
                release.run_release(config(), SOURCE, self.api, self.receipt)
        self.assertNotIn("deployed_version", self.receipt)

    def test_different_candidate_namespace_never_promoted(self):
        self.api.candidate["resources"]["bindings"][-1]["namespace_id"] = "different-namespace"
        with self.assertRaises(release.ReleaseError):
            self.run_release()
        self.assertEqual(self.commands, ["dry-run", "upload"])
        self.assertEqual(self.api.active, OLD)
        self.assertEqual(self.receipt["state"], "uploaded_not_promoted")

    def test_different_candidate_origin_never_promoted(self):
        self.api.candidate["resources"]["bindings"][0]["text"] = "changed"
        with self.assertRaises(release.ReleaseError):
            self.run_release()
        self.assertNotIn("promote", self.commands)

    def test_concurrent_change_before_upload_stops(self):
        self.api.change_at = 2
        with self.assertRaises(release.ReleaseError):
            self.run_release()
        self.assertEqual(self.commands, ["dry-run"])

    def test_concurrent_change_before_promotion_stops(self):
        self.api.change_at = 3
        with self.assertRaises(release.ReleaseError):
            self.run_release()
        self.assertEqual(self.commands, ["dry-run", "upload"])

    def test_failed_smoke_does_not_claim_success_or_rollback(self):
        with patch.object(release, "wrangler", side_effect=self.wrangler), \
                patch.object(release, "smoke", side_effect=release.ReleaseError("synthetic edge block")), \
                patch.object(release.time, "sleep"):
            with self.assertRaises(release.ReleaseError):
                release.run_release(config(), SOURCE, self.api, self.receipt)
        self.assertEqual(self.receipt["state"], "deployed_public_checks_pending")
        self.assertEqual(self.receipt["deployed_version"], NEW)
        self.assertEqual(self.commands.count("promote"), 1)


class APITests(unittest.TestCase):
    def test_promote_is_only_the_official_version_deployment_request(self):
        api = release.Cloudflare("a" * 32)
        with patch.object(api, "request", return_value={"id": "synthetic-deployment"}) as request:
            api.promote(NEW, SOURCE)
        suffix, payload = request.call_args.args
        self.assertEqual(suffix, "/scripts/skarma-web/deployments")
        self.assertEqual(payload["strategy"], "percentage")
        self.assertEqual(payload["versions"], [{"version_id": NEW, "percentage": 100}])
        self.assertEqual(set(payload), {"strategy", "versions", "annotations"})

    def test_missing_or_split_deployment_refused(self):
        for deployments in [[], [{"versions": [{"version_id": OLD, "percentage": 50}]}]]:
            api = release.Cloudflare("a" * 32)
            with patch.object(api, "get", return_value={"deployments": deployments}):
                with self.assertRaises(release.ReleaseError):
                    api.current()

    def test_binding_order_does_not_affect_comparison(self):
        other = version()
        other["resources"]["bindings"].reverse()
        self.assertEqual(release.invariant(version()), release.invariant(other))

    def test_public_smoke_matches_built_bytes_and_origin_contract(self):
        with TemporaryDirectory() as folder:
            root = Path(folder)
            dist = root / "apps/client/dist"
            dist.mkdir(parents=True)
            index = b'<html><script src="/app.js"></script></html>'
            (dist / "index.html").write_bytes(index)
            (dist / "app.js").write_bytes(b"synthetic-js")

            def request(path, headers=None, method="GET"):
                if path == "/": return 200, index
                if path == "/app.js": return 200, b"synthetic-js"
                if path == "/api/health": return 200, b'{"status":"ok"}'
                if path == "/api/auth/logout":
                    allowed = headers.get("Origin") == release.ORIGIN and "x-skarma-request" in headers
                    return (200 if allowed else 403), b"{}"
                if path == "/api/nonexistent-release-check": return 404, b"{}"
                return 401, b"{}"

            with patch.object(release, "ROOT", root), patch.object(release, "public_request", side_effect=request):
                self.assertEqual(release.smoke()["origins"], "passed")
            with patch.object(release, "ROOT", root), patch.object(release, "public_request", return_value=(403, b"edge1010")):
                with self.assertRaises(release.ReleaseError):
                    release.smoke()


class VerificationTests(unittest.TestCase):
    def test_only_known_analytics_tag_and_its_newline_are_normalized(self):
        original = b'<html><script src="/app.js"></script></html>'
        beacon = b'<script defer src="https://static.cloudflareinsights.com/beacon.min.js/v123" data-cf-beacon="synthetic"></script>\n'
        modified = original.replace(b'</html>', beacon + b'</html>')
        self.assertEqual(release.normalize_homepage(modified), (original, 1))
        unknown = b'<script src="https://unknown.example/app.js"></script>'
        self.assertEqual(release.normalize_homepage(unknown), (unknown, 0))
        fake = b'<script data-src="https://static.cloudflareinsights.com/beacon.min.js/v123" src="https://unknown.example/app.js"></script>'
        self.assertEqual(release.normalize_homepage(fake), (fake, 0))
        self.assertNotEqual(release.normalize_homepage(modified.replace(b'/app.js', b'/changed.js'))[0], original)
        with self.assertRaises(release.ReleaseError):
            release.normalize_homepage(beacon + modified)

    def test_existing_version_verification_has_no_write_or_upload(self):
        api = FakeAPI()
        api.old["annotations"] = {"workers/message": "GitHub " + SOURCE}
        receipt = {}
        with patch.object(release, "wrangler", side_effect=AssertionError("must not upload")), \
                patch.object(api, "promote", side_effect=AssertionError("must not promote")), \
                patch.object(release, "smoke", return_value={"synthetic": "passed"}):
            release.verify_existing(config(), OLD, api, receipt)
        self.assertEqual(receipt["state"], "verified_existing_version")
        self.assertEqual(receipt["deployed_version"], OLD)
        self.assertEqual(receipt["deployed_source_sha"], SOURCE)
        self.assertNotIn("uploaded_version", receipt)

    def test_verify_wrong_active_version_stops_before_public_request(self):
        with patch.object(release, "smoke") as smoke:
            with self.assertRaises(release.ReleaseError):
                release.verify_existing(config(), NEW, FakeAPI(), {})
            smoke.assert_not_called()

    def test_verify_concurrent_change_does_not_claim_success(self):
        api = FakeAPI()
        api.change_at = 2
        receipt = {}
        with patch.object(release, "smoke", return_value={"synthetic": "passed"}):
            with self.assertRaises(release.ReleaseError):
                release.verify_existing(config(), OLD, api, receipt)
        self.assertEqual(receipt["state"], "deployed_public_checks_pending")

    def test_monitor_has_truthful_user_agent_and_no_credentials(self):
        from unittest.mock import MagicMock
        response = MagicMock()
        response.__enter__.return_value = response
        response.status = 200
        response.read.return_value = b"synthetic"
        opener = MagicMock()
        opener.open.return_value = response
        with patch.object(release.urllib.request, "build_opener", return_value=opener):
            release.public_request("/")
        request = opener.open.call_args.args[0]
        self.assertEqual(request.get_header("User-agent"), "SkarmaReleaseCheck/1.0")
        self.assertIsNone(request.get_header("Authorization"))
        self.assertIsNone(request.get_header("Cookie"))

    def test_verify_marker_requires_one_exact_uuid(self):
        with TemporaryDirectory() as folder:
            p = Path(folder) / "event.json"
            with patch.dict(release.os.environ, {"GITHUB_EVENT_PATH": str(p)}):
                for message in ["normal", "[verify-cloudflare:invalid]", f"[verify-cloudflare:{OLD}] [verify-cloudflare:{NEW}]"]:
                    p.write_text(json.dumps({"head_commit": {"message": message}}))
                    with self.assertRaises(release.ReleaseError):
                        release.verification_version_from_event()
                p.write_text(json.dumps({"head_commit": {"message": f"Verify [verify-cloudflare:{OLD}]"}}))
                self.assertEqual(release.verification_version_from_event(), OLD)


class EmptyPrdReleaseTests(unittest.TestCase):
    def candidate(self, value):
        for key, text in release.PRD_VARS.items():
            value["resources"]["bindings"].append({"type": "plain_text", "name": key, "text": text})
        return value

    def test_original_secret_accepts_only_exact_reviewed_live_overlay(self):
        release.validate_live(config(), self.candidate(version()))
        for key in release.PRD_VARS:
            bad = self.candidate(version())
            next(b for b in bad["resources"]["bindings"] if b["name"] == key)["text"] = "wrong"
            with self.assertRaises(release.ReleaseError):
                release.validate_live(config(), bad)
        bad = self.candidate(version())
        bad["resources"]["bindings"].pop()
        with self.assertRaises(release.ReleaseError):
            release.validate_live(config(), bad)

    def test_config_must_have_both_exact_model_vars_or_neither(self):
        good = config()
        good["vars"].update(release.PRD_VARS)
        release.validate_config(good)
        good["vars"]["SKARMA_PRD_BASELINE"] = "reset"
        with self.assertRaises(release.ReleaseError):
            release.validate_config(good)
        good["vars"].pop("SKARMA_PRD_BASELINE")
        with self.assertRaises(release.ReleaseError):
            release.validate_config(good)

    def run_cutover(self, api, receipt, activate=True):
        def wrangler(args, label):
            if activate:
                for key, value in release.PRD_VARS.items():
                    index = args.index(key + ":" + value)
                    self.assertEqual(args[index - 1], "--var")
            if label == "upload":
                return [{"type": "version-upload", "version_id": NEW}]
            return []
        with patch.object(release, "wrangler", side_effect=wrangler), \
                patch.object(release, "smoke", return_value={"prd_database": "startup_completed"}) as smoke:
            release.run_release(config(), SOURCE, api, receipt, activate_prd_empty=activate)
            smoke.assert_called_once_with(initialize_prd=True)

    def test_cutover_allows_exact_two_vars_and_preserves_namespace(self):
        api = FakeAPI()
        self.candidate(api.candidate)
        receipt = {}
        self.run_cutover(api, receipt)
        self.assertEqual(receipt["state"], "deployed_and_verified")
        self.assertEqual(receipt["model"], "prd_v1")
        self.assertEqual(api.commands, ["promote"])

    def test_cutover_does_not_allow_any_additional_binding_change(self):
        for mutation in [lambda v: v["resources"]["bindings"][0].update(text="other"),
                         lambda v: v["resources"]["bindings"][-3].update(namespace_id="other")]:
            api = FakeAPI()
            self.candidate(api.candidate)
            mutation(api.candidate)
            with self.assertRaises(release.ReleaseError):
                self.run_cutover(api, {})
            self.assertEqual(api.active, OLD)
            self.assertEqual(api.commands, [])

    def test_ordinary_release_cannot_silently_activate_new_model(self):
        api = FakeAPI()
        self.candidate(api.candidate)
        with patch.object(release, "wrangler", side_effect=lambda args, label: [{"type":"version-upload","version_id":NEW}] if label == "upload" else []):
            with self.assertRaises(release.ReleaseError):
                release.run_release(config(), SOURCE, api, {})
        self.assertEqual(api.active, OLD)

    def test_future_ordinary_release_retains_new_model_from_live(self):
        api = FakeAPI()
        self.candidate(api.old)
        self.candidate(api.candidate)
        receipt = {}
        self.run_cutover(api, receipt, activate=False)
        self.assertEqual(receipt["model"], "prd_v1")
        self.assertEqual(receipt["state"], "deployed_and_verified")

    def test_repeated_explicit_cutover_refused_before_build_or_upload(self):
        api = FakeAPI()
        self.candidate(api.old)
        with patch.object(release, "wrangler") as wrangler:
            with self.assertRaises(release.ReleaseError):
                release.run_release(config(), SOURCE, api, {}, activate_prd_empty=True)
            wrangler.assert_not_called()

    def test_database_probe_uses_unissued_cookie_and_checks_response(self):
        # Reuse the normal public checks, but isolate the additional DB startup
        # probe so its cookie is never an existing user session or a login.
        with TemporaryDirectory() as folder:
            root=Path(folder)
            dist=root / "apps/client/dist"
            dist.mkdir(parents=True)
            (dist / "index.html").write_bytes(b'<script src="/app.js"></script>')
            (dist / "app.js").write_bytes(b'synthetic')
            observed=[]
            def request(path, headers=None, method="GET"):
                if path=="/": return 200,(dist / "index.html").read_bytes()
                if path=="/app.js": return 200,b'synthetic'
                if path=="/api/health": return 200,b'{"status":"ok"}'
                if path=="/api/nonexistent-release-check": return 404,b'{}'
                if path=="/api/auth/logout":
                    if headers.get("Cookie"):
                        observed.append(headers["Cookie"])
                        return 200,b'{"ok":true}'
                    allowed=headers.get("Origin")==release.ORIGIN and headers.get("x-skarma-request")=="1"
                    return (200 if allowed else 403),b'{}'
                return 401,b'{}'
            with patch.object(release,"ROOT",root),patch.object(release,"public_request",side_effect=request):
                self.assertEqual(release.smoke(initialize_prd=True)["prd_database"],"startup_completed")
            self.assertEqual(len(observed),1)
            self.assertRegex(observed[0],r'^__Host-skarma_session=[0-9a-f]{64}$')


if __name__ == "__main__":
    unittest.main()
