# Security

## Scope

Nexus is a local-first inventory and planning tool for Git repositories.

## Reporting

Please report security issues privately before opening a public issue.

## Security priorities

- accidental secret exposure in scanned content
- unsafe command execution
- path traversal / filesystem safety
- destructive repo mutations
- misleading canonical selection that could trigger unsafe follow-up automation
