#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 aesc silicon
# SPDX-License-Identifier: AGPL-3.0-or-later
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
exec python3 "$here/asap7/compare.py" "$@"
