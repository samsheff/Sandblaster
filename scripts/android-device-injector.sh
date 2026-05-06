#!/usr/bin/env bash
set -euo pipefail

DEVICE_DIR="${SANDBLASTER_ANDROID_DIR:-/data/local/tmp/sandblaster}"
adb shell "cd '${DEVICE_DIR}' && './injector' --target android-arm64 $*"
