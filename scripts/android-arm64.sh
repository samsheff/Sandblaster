#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
TARGET="${SANDBLASTER_ANDROID_TARGET:-aarch64-linux-android}"
DEVICE_DIR="${SANDBLASTER_ANDROID_DIR:-/data/local/tmp/sandblaster}"
TIMEOUT_DURATION="${SANDBLASTER_ANDROID_TIMEOUT:-20}"
ANDROID_API="${SANDBLASTER_ANDROID_API:-24}"

usage() {
    cat <<USAGE
Usage: $(basename "$0") <check|build|push|smoke|exec-smoke|injector|sifter> [args...]

Commands:
  check        Type-check the Android ARM64 injector.
  build        Build the Android ARM64 injector.
  push         Push injector to the device.
  smoke        Run an ARM64 dry-run probe on the device.
  exec-smoke   Run one native ARM64 probe on the device.
  injector     Run the device injector with the remaining arguments.
  sifter       Run local sifter against the device injector.

Environment:
  SANDBLASTER_ANDROID_TARGET   Rust target (default: aarch64-linux-android).
  SANDBLASTER_ANDROID_DIR      Device directory (default: /data/local/tmp/sandblaster).
  SANDBLASTER_ANDROID_TIMEOUT  Device smoke timeout seconds (default: 20).
  SANDBLASTER_ANDROID_API      Android API level for NDK clang (default: 24).
USAGE
}

require_adb() {
    if ! command -v adb >/dev/null 2>&1; then
        echo "error: adb is required" >&2
        exit 2
    fi
}

require_cargo() {
    if ! command -v cargo >/dev/null 2>&1; then
        echo "error: cargo is required" >&2
        exit 2
    fi
}

configure_android_linker() {
    if [[ -n "${CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER:-}" ]]; then
        return
    fi

    local sdk ndk_root linker
    sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-${HOME}/Library/Android/sdk}}"
    ndk_root="${ANDROID_NDK_HOME:-}"
    if [[ -z "${ndk_root}" ]]; then
        ndk_root="$(find "${sdk}/ndk" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | sort -V | tail -n 1 || true)"
    fi
    linker="$(find "${ndk_root}/toolchains/llvm/prebuilt" -path "*/bin/aarch64-linux-android${ANDROID_API}-clang" -type f 2>/dev/null | head -n 1 || true)"
    if [[ -z "${linker}" ]]; then
        echo "error: could not find aarch64-linux-android${ANDROID_API}-clang; set ANDROID_NDK_HOME or CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER" >&2
        exit 2
    fi
    export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="${linker}"
}

device_injector="${DEVICE_DIR}/injector"
host_injector="${REPO_ROOT}/target/${TARGET}/debug/injector"

command="${1:-}"
if [[ $# -gt 0 ]]; then
    shift
fi

case "${command}" in
    -h|--help|help)
        usage
        exit 0
        ;;
esac

require_cargo
configure_android_linker
cd "${REPO_ROOT}"

case "${command}" in
    check)
        cargo check --target "${TARGET}" -p sandblaster-injector
        ;;
    build)
        cargo build --target "${TARGET}" -p sandblaster-injector
        ;;
    push)
        require_adb
        cargo build --target "${TARGET}" -p sandblaster-injector
        adb shell "mkdir -p '${DEVICE_DIR}'"
        adb push "${host_injector}" "${device_injector}"
        adb shell "chmod 755 '${device_injector}'"
        ;;
    smoke)
        require_adb
        "$0" push
        adb shell "cd '${DEVICE_DIR}' && timeout '${TIMEOUT_DURATION}' '${device_injector}' --target android-arm64 --dry-run -R -b -B 4 -i 1f2003d5 -e 1f2003d6"
        ;;
    exec-smoke)
        require_adb
        "$0" push
        adb shell "cd '${DEVICE_DIR}' && timeout '${TIMEOUT_DURATION}' '${device_injector}' --target android-arm64 -R -b -B 4 -i 1f2003d5 -e 1f2003d6"
        ;;
    injector)
        require_adb
        adb shell "cd '${DEVICE_DIR}' && '${device_injector}' --target android-arm64 $*"
        ;;
    sifter)
        require_adb
        "$0" push
        SANDBLASTER_INJECTOR="${SCRIPT_DIR}/android-device-injector.sh" \
            cargo run -p sandblaster-cli --bin sifter -- "$@"
        ;;
    "")
        usage >&2
        exit 2
        ;;
    *)
        usage >&2
        exit 2
        ;;
esac
