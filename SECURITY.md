# Security policy

## Reporting a vulnerability

Do not open a public issue for a vulnerability.

Report security problems privately to the repository owner through GitHub's private vulnerability reporting, when enabled. If that feature is unavailable, contact the maintainer through the email address on the maintainer's GitHub profile.

Include:

- affected version or commit
- operating system
- reproduction steps or proof of concept
- impact
- suggested mitigation, if known

Please do not include real credentials, private skill content, or personal paths in reports.

## Scope

Security-sensitive areas include:

- filesystem writes, deletion, and symlink creation
- subprocess and model-server management
- handling of configuration and runtime logs
- importing content from external directories

This project does not provide or manage model-provider credentials. Check external harnesses and model runtimes separately.
