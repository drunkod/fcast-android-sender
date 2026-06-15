# Security Policy

## Supported versions

The `main` branch receives security fixes. Tagged releases are fixed on a
best-effort basis.

## Reporting a vulnerability

Please report security issues privately via GitHub's "Report a vulnerability"
(Security Advisories) on this repository, rather than opening a public issue.
Include reproduction steps and the affected commit or tag. We aim to acknowledge
within 72 hours.

## Scope notes

This app bridges Android (JNI) to a Rust core and a GStreamer/WebRTC media
pipeline. Reports about memory safety at the JNI boundary, signalling/WHEP
handling, or secret storage (`AndroidSecretStore`) are especially welcome.
