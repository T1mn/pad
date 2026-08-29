#!/usr/bin/env python3
"""Black-box smoke tests for the packaged PAD Desktop control plane.

The test intentionally uses a temporary PAD_DESKTOP_DATA_DIR.  It verifies
that the app bundle is arm64/signed, that the bundled Pi can answer RPC state
commands, that Profile roots do not collide, and that PAD metadata survives a
server restart.  Provider-backed prompts are covered separately by
pi_rpc_prompt_smoke.mjs because they consume quota and require credentials.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import pathlib
import platform
import subprocess
import sys
import tempfile
import time
import shlex
from typing import Any


ROOT = pathlib.Path(__file__).resolve().parents[2]
DEFAULT_APP = ROOT / "apps/pad-desktop/dist/PAD Desktop.app"


def command_output(args: list[str], **kwargs: Any) -> str:
    result = subprocess.run(args, check=True, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, **kwargs)
    return result.stdout.strip()


class Server:
    def __init__(self, pad: pathlib.Path, data_root: pathlib.Path) -> None:
        env = os.environ.copy()
        env["PAD_DESKTOP_DATA_DIR"] = str(data_root)
        self.process = subprocess.Popen(
            [str(pad), "__internal", "desktop-server"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=env,
        )

    def request(self, action: str, **fields: Any) -> dict[str, Any]:
        if self.process.stdin is None or self.process.stdout is None:
            raise RuntimeError("desktop-server pipes are unavailable")
        request = {"id": f"e2e-{time.time_ns()}", "action": action, **fields}
        self.process.stdin.write(json.dumps(request) + "\n")
        self.process.stdin.flush()
        line = self.process.stdout.readline()
        if not line:
            stderr = self.process.stderr.read() if self.process.stderr else ""
            raise RuntimeError(f"desktop-server exited without response: {stderr}")
        response = json.loads(line)
        if response.get("ok") is not True:
            raise RuntimeError(f"{action} failed: {response}")
        return response["result"]

    def stop(self) -> None:
        if self.process.poll() is not None:
            return
        try:
            self.request("shutdown")
            self.process.wait(timeout=3)
        except Exception:
            self.process.terminate()
            try:
                self.process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait()


def check_bundle(app: pathlib.Path) -> tuple[pathlib.Path, pathlib.Path]:
    app = app.expanduser().resolve()
    if platform.system() != "Darwin":
        raise RuntimeError("PAD Desktop app smoke is macOS-only")
    if platform.machine() != "arm64":
        raise RuntimeError(f"expected an arm64 smoke host, found {platform.machine()}")
    plist_path = app / "Contents/Info.plist"
    if not plist_path.is_file():
        raise RuntimeError(f"missing app Info.plist: {plist_path}")
    bundle_executable = command_output(["plutil", "-extract", "CFBundleExecutable", "raw", "-o", "-", str(plist_path)])
    if bundle_executable != "PADDesktop":
        raise RuntimeError("Info.plist has an unexpected executable")
    pad = app / "Contents/Resources/pad"
    pi = app / "Contents/Resources/bin/pi"
    node = app / "Contents/Resources/bin/node"
    for path in (app / "Contents/MacOS/PADDesktop", pad, pi):
        if not path.is_file() or not os.access(path, os.X_OK):
            raise RuntimeError(f"missing executable bundle member: {path}")
    if not node.is_file() or not os.access(node, os.X_OK):
        raise RuntimeError(f"missing bundled Node runtime: {node}")
    command_output(["plutil", "-lint", str(plist_path)])
    command_output(["codesign", "--verify", "--deep", "--strict", str(app)])
    if command_output([str(pi), "--version"]) != "0.84.4":
        raise RuntimeError("bundled Pi is not version 0.84.4")
    return pad, pi


def check_control_plane(pad: pathlib.Path) -> None:
    with tempfile.TemporaryDirectory(prefix="pad-desktop-e2e-") as temporary:
        data_root = pathlib.Path(temporary) / "data"
        server = Server(pad, data_root)
        try:
            assert server.request("ping")["protocol_version"] == 1
            bootstrap = server.request("bootstrap")
            default = bootstrap["profile"]
            assert default["policy"]["mode"] == "system_full"
            assert default["policy"]["unattended"] is True
            default_project = bootstrap["records"]["projects"][0]
            assert default_project["primary_root"] not in ("", "/")

            profile_a = server.request("create_profile", profile_id="profile-a", name="A")["profile"]
            profile_b = server.request("create_profile", profile_id="profile-b", name="B")["profile"]
            assert profile_a["agent_dir"] != profile_b["agent_dir"]
            assert profile_a["session_dir"] != profile_b["session_dir"]
            assert profile_a["agent_dir"].startswith(str(data_root))
            assert profile_b["agent_dir"].startswith(str(data_root))

            selected_root = pathlib.Path(temporary) / "selected-project"
            selected_root.mkdir()
            project = server.request(
                "create_project",
                profile_id="profile-a",
                name="selected project",
                cwd=str(selected_root),
            )["project"]
            assert project["primary_root"] == str(selected_root)
            assert project["profile_id"] == "profile-a"

            guarded = server.request(
                "set_profile",
                profile_id="profile-a",
                permission_mode="guarded",
                unattended=False,
            )["profile"]
            assert guarded["policy"]["mode"] == "guarded"
            assert guarded["policy"]["unattended"] is False

            task_a = server.request(
                "create_task",
                profile_id="profile-a",
                project_id=project["id"],
                task_id="task-a",
                title="A task",
                cwd=temporary,
            )["task"]
            task_b = server.request(
                "create_task",
                profile_id="profile-b",
                task_id="task-b",
                title="B task",
                cwd=temporary,
            )["task"]
            for task in (task_a, task_b):
                server.request("start_task", task_id=task["id"], command="/bin/sh -c 'sleep 30'")
            assert pathlib.Path(profile_a["agent_dir"]).is_dir()
            assert pathlib.Path(profile_b["agent_dir"]).is_dir()

            flagged = server.request(
                "set_task",
                task_id="task-a",
                pinned=True,
                archived=True,
                unread=True,
            )["task"]
            assert flagged["pinned"] and flagged["archived"] and flagged["unread"]
            assert server.request("stop_task", task_id="task-a")["stopped"] is True
            assert server.request("stop_task", task_id="task-b")["stopped"] is True
        finally:
            server.stop()

        reopened = Server(pad, data_root)
        try:
            records = reopened.request("bootstrap")["records"]
            restored = next(task for task in records["tasks"] if task["id"] == "task-a")
            assert restored["pinned"] and restored["archived"] and restored["unread"]
        finally:
            reopened.stop()


def check_session_restore(pi: pathlib.Path) -> None:
    with tempfile.TemporaryDirectory(prefix="pad-pi-history-") as temporary:
        root = pathlib.Path(temporary)
        agent = root / "agent"
        sessions = root / "sessions"
        agent.mkdir()
        sessions.mkdir()
        timestamp = dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z")
        session = sessions / "fixture.jsonl"
        entries = [
            {"type": "session", "version": 3, "id": "history-fixture", "timestamp": timestamp, "cwd": temporary},
            {
                "type": "message",
                "id": "entry-user",
                "parentId": None,
                "timestamp": timestamp,
                "message": {
                    "role": "user",
                    "content": [{"type": "text", "text": "PAD_HISTORY_FIXTURE"}],
                    "timestamp": int(time.time() * 1000),
                },
            },
        ]
        session.write_text("".join(json.dumps(entry) + "\n" for entry in entries), encoding="utf-8")
        env = os.environ.copy()
        env.update({"PI_CODING_AGENT_DIR": str(agent), "PI_CODING_AGENT_SESSION_DIR": str(sessions)})
        process = subprocess.Popen(
            [
                str(pi),
                "--mode",
                "rpc",
                "--offline",
                "--session",
                str(session),
                "--session-dir",
                str(sessions),
                "--no-approve",
                "--no-context-files",
                "--no-extensions",
                "--no-skills",
                "--no-prompt-templates",
                "--no-themes",
            ],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=env,
        )
        try:
            assert process.stdin is not None and process.stdout is not None
            process.stdin.write('{"id":"entries","type":"get_entries"}\n')
            process.stdin.flush()
            entries_response = json.loads(process.stdout.readline())
            assert entries_response["success"] is True
            assert len(entries_response["data"]["entries"]) == 2
            process.stdin.write('{"id":"messages","type":"get_messages"}\n')
            process.stdin.flush()
            messages_response = json.loads(process.stdout.readline())
            messages = messages_response["data"]["messages"]
            assert any("PAD_HISTORY_FIXTURE" in json.dumps(message) for message in messages)
        finally:
            process.terminate()
            process.wait(timeout=3)


def check_bundled_pi_server(pad: pathlib.Path, pi: pathlib.Path) -> None:
    """Start the actual bundled Pi through the actual Desktop server."""

    with tempfile.TemporaryDirectory(prefix="pad-bundled-pi-e2e-") as temporary:
        root = pathlib.Path(temporary)
        env = os.environ.copy()
        env.update({"PAD_DESKTOP_DATA_DIR": str(root / "data"), "HOME": str(root / "home")})
        pathlib.Path(env["HOME"]).mkdir()
        server = Server(pad, root / "data")
        # Server() inherits the normal environment; the PAD data root is
        # already isolated by its constructor.  The explicit pi path below
        # makes this test independent of the developer's PATH.
        try:
            profile = server.request("bootstrap")["profile"]
            task = server.request(
                "create_task",
                profile_id=profile["id"],
                task_id="task-bundled-pi",
                title="bundled Pi",
                cwd=temporary,
            )["task"]
            # The supervisor accepts shell-word syntax so paths containing the
            # app's "PAD Desktop.app" bundle name must be quoted.
            server.request("start_task", task_id=task["id"], command=shlex.join([str(pi)]))
            server.request("prompt", task_id=task["id"], prompt="PAD_BUNDLED_PI_NO_CREDENTIALS")
            prompt_error = False
            deadline = time.monotonic() + 3
            while time.monotonic() < deadline:
                poll = server.request("poll", task_id=task["id"])["poll"]
                for message in poll["messages"]:
                    value = message.get("value")
                    if isinstance(value, dict) and value.get("command") == "prompt":
                        prompt_error = value.get("success") is False
                        break
                if prompt_error:
                    break
                time.sleep(0.05)
            assert prompt_error, "bundled Pi did not return the expected credential error"
            server.request("stop_task", task_id=task["id"])
        finally:
            server.stop()


def check_full_access_ui_policy(pad: pathlib.Path) -> None:
    """Exercise UI-request policy with a fake Pi sidecar.

    Full Access may auto-confirm an explicit permission confirmation, but it
    must never guess a business select/input answer.  This uses the same
    desktop-server bridge as Swift and avoids a model call.
    """

    fake_source = """#!/usr/bin/env python3
import json
import sys
import time

kind = sys.argv[1]
if kind == "confirm":
    request = {"type": "extension_ui_request", "method": "confirm", "id": "confirm-1", "title": "Allow tool execution?", "message": "Permit this command?"}
elif kind == "select":
    request = {"type": "extension_ui_request", "method": "select", "id": "select-1", "title": "Choose a deployment", "options": ["staging", "production"], "defaultIndex": 1}
else:
    request = {"type": "extension_ui_request", "method": "input", "id": "input-1", "title": "Enter release note", "default": "draft"}
print(json.dumps(request), flush=True)
for line in sys.stdin:
    try:
        value = json.loads(line)
    except json.JSONDecodeError:
        continue
    # DesktopRuntime sends native get_state during startup. Only an actual
    # extension response proves that Full Access answered the UI request.
    if value.get("type") == "extension_ui_response":
        valid = kind != "confirm" or (value.get("confirmed") is True and "value" not in value)
        print(json.dumps({"type": "received" if valid else "invalid", "request": value}), flush=True)
        break
time.sleep(10)
"""

    def run_case(kind: str, expect_response: bool) -> None:
        with tempfile.TemporaryDirectory(prefix="pad-approval-e2e-") as temporary:
            root = pathlib.Path(temporary)
            fake = root / "fake_pi.py"
            fake.write_text(fake_source, encoding="utf-8")
            fake.chmod(0o755)
            server = Server(pad, root / "data")
            try:
                profile = server.request(
                    "create_profile",
                    profile_id=f"profile-{kind}",
                    name=f"{kind} profile",
                    permission_mode="system_full",
                    unattended=True,
                )["profile"]
                task = server.request(
                    "create_task",
                    profile_id=profile["id"],
                    task_id=f"task-{kind}",
                    title=f"{kind} task",
                    cwd=temporary,
                )["task"]
                command = shlex.join([sys.executable, str(fake), kind])
                server.request("start_task", task_id=task["id"], command=command)
                received = False
                deadline = time.monotonic() + 2
                while time.monotonic() < deadline:
                    poll = server.request("poll", task_id=task["id"])["poll"]
                    if any(message.get("type") == "received" for message in poll["messages"]):
                        received = True
                        break
                    time.sleep(0.05)
                if received != expect_response:
                    raise RuntimeError(
                        f"Full Access {kind} policy mismatch: expected response={expect_response}, got {received}"
                    )
            finally:
                server.stop()

    run_case("confirm", expect_response=True)
    run_case("select", expect_response=False)
    run_case("input", expect_response=False)


def check_guarded_approval_round_trip(pad: pathlib.Path) -> None:
    """Verify guarded mode exposes a request and forwards an explicit answer."""

    fake_source = """#!/usr/bin/env python3
import json
import sys
import time

print(json.dumps({
    "type": "extension_ui_request",
    "method": "confirm",
    "id": "guarded-confirm-1",
    "title": "Allow tool execution?",
    "message": "Permit this command?",
}), flush=True)
for line in sys.stdin:
    try:
        value = json.loads(line)
    except json.JSONDecodeError:
        continue
    if value.get("type") == "extension_ui_response":
        valid = value.get("confirmed") is True and "value" not in value
        print(json.dumps({"type": "received" if valid else "invalid", "request": value}), flush=True)
        break
time.sleep(10)
"""

    with tempfile.TemporaryDirectory(prefix="pad-guarded-e2e-") as temporary:
        root = pathlib.Path(temporary)
        fake = root / "fake_pi.py"
        fake.write_text(fake_source, encoding="utf-8")
        fake.chmod(0o755)
        server = Server(pad, root / "data")
        try:
            profile = server.request(
                "create_profile",
                profile_id="profile-guarded",
                name="guarded profile",
                permission_mode="guarded",
                unattended=False,
            )["profile"]
            task = server.request(
                "create_task",
                profile_id=profile["id"],
                task_id="task-guarded",
                title="guarded task",
                cwd=temporary,
            )["task"]
            command = shlex.join([sys.executable, str(fake)])
            server.request("start_task", task_id=task["id"], command=command)

            request_id = None
            deadline = time.monotonic() + 2
            while time.monotonic() < deadline and request_id is None:
                poll = server.request("poll", task_id=task["id"])["poll"]
                for message in poll["messages"]:
                    if message.get("type") != "extension_ui_request":
                        continue
                    value = message.get("value")
                    if isinstance(value, dict):
                        request_id = value.get("id")
                        break
                assert not any(message.get("type") == "received" for message in poll["messages"]), (
                    "guarded mode answered a request before explicit user response"
                )
                if request_id is None:
                    time.sleep(0.05)
            if request_id != "guarded-confirm-1":
                raise RuntimeError(f"guarded approval request was not surfaced: {request_id!r}")

            response = server.request(
                "extension_ui_response",
                task_id=task["id"],
                interaction_id=request_id,
                response_kind="confirm",
                value=True,
            )
            assert response["accepted"] is True

            received = False
            deadline = time.monotonic() + 2
            while time.monotonic() < deadline:
                poll = server.request("poll", task_id=task["id"])["poll"]
                if any(message.get("type") == "received" for message in poll["messages"]):
                    received = True
                    break
                time.sleep(0.05)
            assert received, "guarded approval response did not reach Pi"
        finally:
            server.stop()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--app", type=pathlib.Path, default=DEFAULT_APP)
    parser.add_argument("--pad-bin", type=pathlib.Path)
    parser.add_argument("--pi-bin", type=pathlib.Path)
    args = parser.parse_args()
    try:
        pad, bundled_pi = check_bundle(args.app)
        if args.pad_bin:
            pad = args.pad_bin.expanduser().resolve()
        pi = args.pi_bin.expanduser().resolve() if args.pi_bin else bundled_pi
        check_control_plane(pad)
        check_session_restore(pi)
        check_bundled_pi_server(pad, pi)
        check_full_access_ui_policy(pad)
        check_guarded_approval_round_trip(pad)
    except (AssertionError, OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(f"pad_desktop_e2e_smoke: failed: {error}", file=sys.stderr)
        return 1
    print("pad_desktop_e2e_smoke: bundle/control-plane/profile-isolation/stop/reopen/history passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
