#!/usr/bin/env python3
"""Black-box PAD Remote v1 acceptance over a real TLS WebSocket.

The test uses only the Python standard library and a temporary PAD data root.
It never opens the user's PAD, Pi, Codex, or ChatGPT data. Model inference is
not required: the acceptance target is transport, isolation, recovery, and
command idempotency.
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import pathlib
import secrets
import socket
import ssl
import struct
import subprocess
import tempfile
import time
import urllib.parse
from typing import Any


ROOT = pathlib.Path(__file__).resolve().parents[2]
DEFAULT_PAD = ROOT / "rust-tui/target/debug/pad"
SUBPROTOCOL = "pad.remote.v1"
MAX_FRAME_BYTES = 1024 * 1024


class AcceptanceError(RuntimeError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AcceptanceError(message)


class DesktopServer:
    def __init__(self, pad: pathlib.Path, data_root: pathlib.Path) -> None:
        environment = os.environ.copy()
        environment["PAD_DESKTOP_DATA_DIR"] = str(data_root)
        self.process = subprocess.Popen(
            [str(pad), "__internal", "desktop-server"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=environment,
        )
        self.events: list[dict[str, Any]] = []

    def request(self, action: str, *, legacy: bool = False, **fields: Any) -> dict[str, Any]:
        require(self.process.stdin is not None, "desktop-server stdin is unavailable")
        require(self.process.stdout is not None, "desktop-server stdout is unavailable")
        request_id = f"remote-e2e-{time.time_ns()}"
        request = {"id": request_id, "action": action, **fields}
        if not legacy:
            request["protocol_version"] = 2
        self.process.stdin.write(json.dumps(request, separators=(",", ":")) + "\n")
        self.process.stdin.flush()
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            line = self.process.stdout.readline()
            if not line:
                stderr = self.process.stderr.read() if self.process.stderr else ""
                raise AcceptanceError(f"desktop-server exited during {action}: {stderr}")
            frame = json.loads(line)
            if frame.get("id") != request_id:
                self.events.append(frame)
                continue
            require(frame.get("ok") is True, f"desktop action {action} failed: {frame}")
            result = frame.get("result")
            require(isinstance(result, dict), f"desktop action {action} returned no object")
            return result
        raise AcceptanceError(f"desktop action {action} timed out")

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
                self.process.wait(timeout=2)


class WebSocketClient:
    def __init__(self, endpoint: str, fingerprint: str) -> None:
        parsed = urllib.parse.urlsplit(endpoint)
        require(parsed.scheme == "wss", f"pairing endpoint is not WSS: {endpoint}")
        require(parsed.hostname is not None and parsed.port is not None, "WSS endpoint is incomplete")
        self.host = parsed.hostname
        self.port = parsed.port
        self.path = parsed.path or "/"
        if parsed.query:
            self.path += f"?{parsed.query}"

        context = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
        context.check_hostname = False
        context.verify_mode = ssl.CERT_NONE
        raw = self._connect_socket(self.host, self.port)
        raw.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
        self.socket = context.wrap_socket(raw, server_hostname=self.host)
        certificate = self.socket.getpeercert(binary_form=True)
        actual = hashlib.sha256(certificate).hexdigest()
        require(actual == fingerprint, f"leaf DER fingerprint mismatch: {actual}")
        self.socket.settimeout(4)
        self._handshake()
        self.events: list[dict[str, Any]] = []

    @staticmethod
    def _connect_socket(host: str, port: int) -> socket.socket:
        try:
            return socket.create_connection((host, port), timeout=4)
        except OSError:
            # The gateway advertises a stable mDNS name. Some headless CI
            # runners do not run mDNS resolution, but the server is local.
            return socket.create_connection(("127.0.0.1", port), timeout=4)

    def _handshake(self) -> None:
        key = base64.b64encode(secrets.token_bytes(16)).decode("ascii")
        authority = f"{self.host}:{self.port}"
        request = (
            f"GET {self.path} HTTP/1.1\r\n"
            f"Host: {authority}\r\n"
            "Upgrade: websocket\r\n"
            "Connection: Upgrade\r\n"
            f"Sec-WebSocket-Key: {key}\r\n"
            "Sec-WebSocket-Version: 13\r\n"
            f"Sec-WebSocket-Protocol: {SUBPROTOCOL}\r\n\r\n"
        )
        self.socket.sendall(request.encode("ascii"))
        response = self._read_until(b"\r\n\r\n", 16 * 1024)
        header_text = response.decode("latin-1")
        require(header_text.startswith("HTTP/1.1 101"), f"WebSocket upgrade failed: {header_text}")
        headers = {}
        for line in header_text.split("\r\n")[1:]:
            if ":" in line:
                name, value = line.split(":", 1)
                headers[name.strip().lower()] = value.strip()
        expected = base64.b64encode(
            hashlib.sha1((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode("ascii")).digest()
        ).decode("ascii")
        require(headers.get("sec-websocket-accept") == expected, "invalid WebSocket accept proof")
        require(headers.get("sec-websocket-protocol") == SUBPROTOCOL, "remote subprotocol was not negotiated")

    def _read_until(self, marker: bytes, limit: int) -> bytes:
        value = bytearray()
        while marker not in value:
            chunk = self.socket.recv(4096)
            require(bool(chunk), "connection closed during WebSocket handshake")
            value.extend(chunk)
            require(len(value) <= limit, "WebSocket handshake exceeded its size limit")
        return bytes(value)

    def send_json(self, value: dict[str, Any]) -> None:
        payload = json.dumps(value, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
        require(len(payload) <= MAX_FRAME_BYTES, "test attempted to send an oversized frame")
        mask = secrets.token_bytes(4)
        header = bytearray([0x81])
        length = len(payload)
        if length < 126:
            header.append(0x80 | length)
        elif length <= 0xFFFF:
            header.append(0x80 | 126)
            header.extend(struct.pack("!H", length))
        else:
            header.append(0x80 | 127)
            header.extend(struct.pack("!Q", length))
        masked = bytes(byte ^ mask[index % 4] for index, byte in enumerate(payload))
        self.socket.sendall(bytes(header) + mask + masked)

    def receive_json(self, timeout_seconds: float = 4) -> dict[str, Any]:
        self.socket.settimeout(timeout_seconds)
        while True:
            opcode, payload = self._receive_frame()
            if opcode == 0x8:
                raise EOFError("WebSocket closed")
            if opcode == 0x9:
                self._send_control(0xA, payload)
                continue
            if opcode == 0xA:
                continue
            require(opcode == 0x1, f"unexpected WebSocket opcode {opcode}")
            require(len(payload) <= MAX_FRAME_BYTES, "server sent an oversized frame")
            value = json.loads(payload.decode("utf-8"))
            require(isinstance(value, dict), "remote frame is not a JSON object")
            if value.get("type") == "ping":
                self.send_json({"type": "pong"})
                continue
            return value

    def command(self, action: str, params: dict[str, Any], command_id: str | None = None) -> tuple[dict[str, Any], float]:
        command_id = command_id or secrets.token_hex(16)
        started = time.perf_counter()
        self.send_json({"type": "command", "command_id": command_id, "action": action, "params": params})
        while True:
            frame = self.receive_json()
            if frame.get("type") == "event":
                self.events.append(frame)
                continue
            if frame.get("type") == "resync_required":
                self.events.append(frame)
                continue
            require(frame.get("type") != "error", f"remote transport error: {frame}")
            if frame.get("type") == "command_result" and frame.get("command_id") == command_id:
                return frame, (time.perf_counter() - started) * 1000

    def wait_for(self, frame_type: str, timeout_seconds: float = 4) -> dict[str, Any]:
        deadline = time.monotonic() + timeout_seconds
        while time.monotonic() < deadline:
            frame = self.receive_json(max(0.05, deadline - time.monotonic()))
            if frame.get("type") == frame_type:
                return frame
            self.events.append(frame)
        raise AcceptanceError(f"timed out waiting for remote {frame_type}")

    def wait_closed(self, timeout_seconds: float = 3) -> None:
        deadline = time.monotonic() + timeout_seconds
        while time.monotonic() < deadline:
            try:
                self.receive_json(max(0.05, deadline - time.monotonic()))
            except (EOFError, OSError, ssl.SSLError):
                return
        raise AcceptanceError("revoked remote connection remained open")

    def close(self) -> None:
        try:
            self._send_control(0x8, b"")
        except OSError:
            pass
        self.socket.close()

    def _receive_frame(self) -> tuple[int, bytes]:
        first, second = self._read_exact(2)
        require(first & 0x80 != 0, "fragmented frames are not supported by Remote v1")
        opcode = first & 0x0F
        masked = second & 0x80 != 0
        length = second & 0x7F
        if length == 126:
            length = struct.unpack("!H", self._read_exact(2))[0]
        elif length == 127:
            length = struct.unpack("!Q", self._read_exact(8))[0]
        require(length <= MAX_FRAME_BYTES, "server frame exceeds 1 MiB")
        mask = self._read_exact(4) if masked else b""
        payload = self._read_exact(length)
        if masked:
            payload = bytes(byte ^ mask[index % 4] for index, byte in enumerate(payload))
        return opcode, payload

    def _send_control(self, opcode: int, payload: bytes) -> None:
        require(len(payload) <= 125, "invalid WebSocket control payload")
        mask = secrets.token_bytes(4)
        masked = bytes(byte ^ mask[index % 4] for index, byte in enumerate(payload))
        self.socket.sendall(bytes([0x80 | opcode, 0x80 | len(payload)]) + mask + masked)

    def _read_exact(self, length: int) -> bytes:
        value = bytearray()
        while len(value) < length:
            chunk = self.socket.recv(length - len(value))
            if not chunk:
                raise EOFError("WebSocket closed")
            value.extend(chunk)
        return bytes(value)


def parse_pairing_uri(uri: str) -> tuple[str, str, str, str]:
    parsed = urllib.parse.urlsplit(uri)
    require((parsed.scheme, parsed.netloc, parsed.path) == ("pad", "remote", "/pair"), "unexpected pairing route")
    pairs = urllib.parse.parse_qsl(parsed.query, keep_blank_values=True)
    require(len({key for key, _ in pairs}) == len(pairs), "pairing URI contains duplicate fields")
    query = dict(pairs)
    require(query.get("v") == "1", "pairing URI version is not v1")
    for field in ("endpoint", "fingerprint", "pairing_id", "secret"):
        require(bool(query.get(field)), f"pairing URI is missing {field}")
    require(len(query["fingerprint"]) == 64, "pairing fingerprint is not SHA-256 hex")
    secret = query["secret"]
    decoded = base64.urlsafe_b64decode(secret + "=" * ((4 - len(secret) % 4) % 4))
    require(len(decoded) == 32 and "=" not in secret, "pairing secret is not 256-bit base64url-no-pad")
    return query["endpoint"], query["fingerprint"], query["pairing_id"], secret


def assert_no_private_fields(value: Any) -> None:
    forbidden = {
        "cwd", "path", "primary_root", "additional_roots", "session_file",
        "session_dir", "agent_dir", "credential_ref", "device_token", "secret",
        "api_key", "authorization", "password", "stderr", "raw_stderr",
    }
    if isinstance(value, dict):
        leaked = forbidden.intersection(key.lower() for key in value)
        require(not leaked, f"remote projection leaked fields: {sorted(leaked)}")
        for nested in value.values():
            assert_no_private_fields(nested)
    elif isinstance(value, list):
        for nested in value:
            assert_no_private_fields(nested)


def pair(client: WebSocketClient, pairing_id: str, secret: str) -> dict[str, Any]:
    client.send_json({
        "type": "pair",
        "pairing_id": pairing_id,
        "secret": secret,
        "device": {"display_name": "PAD Remote E2E", "platform": "ios"},
    })
    paired = client.wait_for("paired")
    require(isinstance(paired.get("device_id"), str), "paired frame has no device id")
    require(isinstance(paired.get("device_token"), str), "paired frame has no device token")
    require(isinstance(paired.get("server_epoch"), str), "paired frame has no server epoch")
    require(isinstance(paired.get("latest_revision"), int), "paired frame has no revision cursor")
    return paired


def resume(client: WebSocketClient, paired: dict[str, Any], epoch: str, revision: int) -> dict[str, Any]:
    client.send_json({
        "type": "resume",
        "device_id": paired["device_id"],
        "device_token": paired["device_token"],
        "server_epoch": epoch,
        "after_revision": revision,
    })
    return client.wait_for("welcome")


def percentile95(values: list[float]) -> float:
    ordered = sorted(values)
    return ordered[max(0, (95 * len(ordered) + 99) // 100 - 1)]


def run(pad: pathlib.Path, maximum_p95_ms: float) -> None:
    require(pad.is_file() and os.access(pad, os.X_OK), f"build PAD first: {pad}")
    with tempfile.TemporaryDirectory(prefix="pad-remote-e2e-") as temporary:
        root = pathlib.Path(temporary)
        data_root = root / "data"
        server = DesktopServer(pad, data_root)
        client: WebSocketClient | None = None
        try:
            bootstrap = server.request("bootstrap")
            profile_a = bootstrap["profile"]
            server.request("remote_set_enabled", enabled=True)
            ticket = server.request("remote_pair_begin")["pairing"]
            endpoint, fingerprint, pairing_id, secret = parse_pairing_uri(ticket["qr_payload"])
            require(ticket["pairing_id"] == pairing_id, "pairing id differs between DTO and QR")

            client = WebSocketClient(endpoint, fingerprint)
            paired = pair(client, pairing_id, secret)
            revision = paired["latest_revision"]
            epoch = paired["server_epoch"]

            first, _ = client.command("bootstrap", {})
            require(first.get("ok") is True, f"remote bootstrap failed: {first}")
            assert_no_private_fields(first.get("result"))

            replacement = WebSocketClient(endpoint, fingerprint)
            replacement_welcome = resume(replacement, paired, epoch, revision)
            require(
                replacement_welcome.get("server_epoch") == epoch,
                "same-device replacement changed the server epoch",
            )
            client.wait_closed()
            client.close()
            client = replacement

            duplicate_id = secrets.token_hex(16)
            created, _ = client.command("create_task", {"title": "Remote idempotency"}, duplicate_id)
            require(created.get("ok") is True, f"remote create_task failed: {created}")
            repeated, _ = client.command("create_task", {"title": "Remote idempotency"}, duplicate_id)
            require(repeated == created, "duplicate command did not return its original receipt")
            conflict, _ = client.command("create_task", {"title": "conflicting UUID reuse"}, duplicate_id)
            require(conflict.get("ok") is False, "same command UUID accepted a different payload")
            require(
                conflict.get("error", {}).get("code") == "command_id_conflict",
                f"conflicting command UUID returned the wrong error: {conflict}",
            )
            task_id = created["result"]["task_id"]
            listed, _ = client.command("list_sidebar", {})
            tasks = listed["result"]["records"]["tasks"]
            require(sum(task.get("id") == task_id for task in tasks) == 1, "duplicate command created two tasks")

            profile_b = server.request("create_profile", legacy=True, profile_id="remote-profile-b", name="Private B")["profile"]
            task_b = server.request(
                "create_task",
                legacy=True,
                profile_id=profile_b["id"],
                task_id="remote-private-task-b",
                title="Private B task",
                cwd=str(root),
            )["task"]
            denied, _ = client.command("history", {"task_id": task_b["id"]})
            require(denied.get("ok") is False, "paired Profile A accessed Profile B")
            forbidden, _ = client.command("auth_begin", {})
            require(forbidden.get("ok") is False, "remote auth action was not denied")

            latencies = []
            for _ in range(25):
                result, elapsed = client.command("list_sidebar", {})
                require(result.get("ok") is True, "latency probe failed")
                latencies.append(elapsed)
            p95 = percentile95(latencies)
            require(p95 < maximum_p95_ms, f"warm LAN command P95 {p95:.1f} ms exceeds {maximum_p95_ms:.1f} ms")

            while True:
                try:
                    event = client.receive_json(0.1)
                except (socket.timeout, TimeoutError):
                    break
                if event.get("type") == "event":
                    revision = max(revision, int(event["revision"]))
                    client.send_json({"type": "ack", "through_revision": revision})

            resume_latencies = []
            for _ in range(10):
                client.close()
                started = time.perf_counter()
                client = WebSocketClient(endpoint, fingerprint)
                hot_welcome = resume(client, paired, epoch, revision)
                require(hot_welcome.get("server_epoch") == epoch, "hot resume changed the server epoch")
                resume_latencies.append((time.perf_counter() - started) * 1_000)
            resume_p95 = percentile95(resume_latencies)
            require(resume_p95 < 1_000, f"foreground hot-resume P95 {resume_p95:.1f} ms exceeds 1000 ms")

            client.close()
            client = None
            server.request("set_task", legacy=True, task_id=task_id, unread=True)

            replay_client = WebSocketClient(endpoint, fingerprint)
            welcome = resume(replay_client, paired, epoch, revision)
            require(welcome.get("server_epoch") == epoch, "same-process resume changed epoch")
            replay = replay_client.wait_for("event")
            require(int(replay["revision"]) > revision, "resume did not replay the missing event")
            revision = int(replay["revision"])
            replay_client.send_json({"type": "ack", "through_revision": revision})
            replay_client.close()

            server.stop()
            server = DesktopServer(pad, data_root)
            status = server.request("remote_status")["remote"]
            require(status["enabled"] is True, "remote enabled state did not survive restart")
            restarted = WebSocketClient(endpoint, fingerprint)
            new_welcome = resume(restarted, paired, epoch, revision)
            require(new_welcome.get("server_epoch") != epoch, "server restart did not rotate epoch")
            restarted.wait_for("resync_required")

            server.request("remote_set_enabled", enabled=False)
            disabled = restarted.wait_for("error")
            require(
                disabled.get("error", {}).get("code") == "remote_disabled",
                f"gateway disable returned the wrong close reason: {disabled}",
            )
            restarted.wait_closed()
            restarted.close()
            server.request("remote_set_enabled", enabled=True)
            restarted = WebSocketClient(endpoint, fingerprint)
            toggled_welcome = resume(
                restarted,
                paired,
                str(new_welcome["server_epoch"]),
                int(new_welcome["latest_revision"]),
            )
            require(
                toggled_welcome.get("server_epoch") == new_welcome.get("server_epoch"),
                "enabling the gateway unexpectedly invalidated the paired device",
            )

            server.request("remote_device_revoke", device_id=paired["device_id"])
            restarted.wait_closed()
            restarted.close()
            rejected = WebSocketClient(endpoint, fingerprint)
            rejected.send_json({
                "type": "resume",
                "device_id": paired["device_id"],
                "device_token": paired["device_token"],
                "server_epoch": new_welcome["server_epoch"],
                "after_revision": 0,
            })
            error = rejected.wait_for("error")
            require(error.get("error", {}).get("code") == "resume_rejected", "revoked token was accepted")
            rejected.close()

            state_path = data_root / "v1" / "remote" / "state.json"
            remote_root = state_path.parent
            root_mode = remote_root.stat().st_mode & 0o777
            require(root_mode == 0o700, f"remote directory permissions are {root_mode:o}, expected 700")
            for private_file in remote_root.iterdir():
                if not private_file.is_file():
                    continue
                mode = private_file.stat().st_mode & 0o777
                require(mode == 0o600, f"{private_file.name} permissions are {mode:o}, expected 600")
            state_text = state_path.read_text(encoding="utf-8")
            require(paired["device_token"] not in state_text, "plaintext device token was persisted")
            require(secret not in state_text, "one-time pairing secret was persisted")
            print(
                "[PASS] PAD Remote TLS/pair/resume/replay/isolation/revoke; "
                f"warm command P95={p95:.1f} ms; hot-resume P95={resume_p95:.1f} ms"
            )
        finally:
            if client is not None:
                client.close()
            server.stop()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--pad", type=pathlib.Path, default=DEFAULT_PAD)
    parser.add_argument("--max-p95-ms", type=float, default=150.0)
    arguments = parser.parse_args()
    try:
        run(arguments.pad.expanduser().resolve(), arguments.max_p95_ms)
    except AcceptanceError as error:
        print(f"[FAIL] {error}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
