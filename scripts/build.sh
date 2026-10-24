#!/usr/bin/env bash
set -euo pipefail
if command -v gradle >/dev/null 2>&1; then exec gradle "$@"; fi
printf '%s\n' 'Install Gradle 8.11.1 + JDK 17 + Android SDK 35, or build with the GitHub Actions workflow.' >&2
exit 1
