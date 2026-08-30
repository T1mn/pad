# Security Policy

## Supported product

Only the current PAD Desktop and PAD Remote code on the main development line is supported.

## Reporting

Please report vulnerabilities privately to the repository owner. Include the affected version, reproduction steps, impact, and any suggested mitigation. Do not publish credentials, device tokens, session files, or provider responses in an issue.

## Security boundaries

- PAD stores data only under its own Application Support directory.
- Each Profile has an isolated Pi agent directory and session directory.
- Renderer code cannot read credentials directly; login and model operations run in the Electron main process.
- Remote pairing uses one-time secrets and per-device tokens. Revoked devices lose access immediately.
- Full Access affects Pi task confirmations only; it does not grant the renderer arbitrary filesystem or process access.
