#!/usr/bin/env bash
# Run ON a dedicated, disposable Debian/Ubuntu x86_64 VPS. No SSH credentials.
# Source and generated evidence stay on that VPS; nothing is published by this script.
set -euo pipefail
SOURCE=$(cd "$(dirname "$0")/.." && pwd)
QA_ROOT=${EASYGOT_QA_ROOT:-"$SOURCE/../easygot-vps-qa"}
mkdir -p "$QA_ROOT/evidence" "$QA_ROOT/downloads" "$QA_ROOT/toolchains"
QA_ROOT=$(cd "$QA_ROOT" && pwd)
exec > >(tee -a "$QA_ROOT/evidence/run.log") 2>&1
EMULATOR_PID=''
cleanup() {
  local result=$?
  trap - EXIT
  if [ -n "$EMULATOR_PID" ]; then
    adb -s emulator-5580 logcat -d > "$QA_ROOT/evidence/last-logcat.txt" 2>&1 || true
    adb -s emulator-5580 emu kill >/dev/null 2>&1 || true
    kill "$EMULATOR_PID" 2>/dev/null || true
  fi
  printf '%s\n' "$result" > "$QA_ROOT/evidence/exit-status.txt"
  date -u +%FT%TZ > "$QA_ROOT/evidence/finished-at.txt"
  tar -C "$QA_ROOT" -czf "$QA_ROOT/EasyGot-vps-evidence.tar.gz" evidence
  printf '\nExit status: %s\nEvidence: %s\n' "$result" "$QA_ROOT/EasyGot-vps-evidence.tar.gz"
  exit "$result"
}
trap cleanup EXIT
cd "$SOURCE"
date -u +%FT%TZ
TOKYO_HOUR=$(TZ=Asia/Tokyo date +%H)
if [ "${TOKYO_HOUR#0}" -ge 21 ]; then echo 'Do not start before the temporary server shutdown window.'; exit 1; fi
git rev-parse HEAD > "$QA_ROOT/evidence/source-commit.txt"
git status --porcelain > "$QA_ROOT/evidence/source-status.txt"
test ! -s "$QA_ROOT/evidence/source-status.txt" || { echo 'Use a clean checkout for reproducible VPS evidence.'; exit 1; }
test "$(uname -m)" = x86_64 || { echo 'This script requires an x86_64 VPS.'; exit 1; }
command -v apt-get >/dev/null || { echo 'This script requires Debian/Ubuntu with apt-get.'; exit 1; }
test "$(id -u)" = 0 || { echo 'Run on the disposable VPS with root privileges to install SDK dependencies.'; exit 1; }
{
  uname -a
  cat /etc/os-release
  nproc
  free -h
  df -h "$QA_ROOT"
  ls -l /dev/kvm 2>/dev/null || true
} > "$QA_ROOT/evidence/host.txt"
AVAILABLE_KB=$(df --output=avail "$QA_ROOT" | tail -1 | tr -d ' ')
test "$AVAILABLE_KB" -ge 25165824 || { echo 'At least 24 GiB of free disk space is required.'; exit 1; }
MEMORY_KB=$(awk '/MemTotal/ {print $2}' /proc/meminfo)
test "$MEMORY_KB" -ge 6000000 || { echo 'At least 6 GB RAM is required for the Rust/Skia build and emulator.'; exit 1; }
export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y ca-certificates curl unzip zip git build-essential pkg-config libfontconfig1-dev \
  openjdk-17-jdk-headless python3-venv tesseract-ocr tesseract-ocr-chi-sim \
  libglu1-mesa libnss3 libx11-6 libxcb1 libxcomposite1 libxcursor1 libxi6 libxtst6 libxrandr2
export JAVA_HOME=/usr/lib/jvm/java-17-openjdk-amd64
export ANDROID_HOME="$QA_ROOT/toolchains/android-sdk"
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/28.2.13676358"
export ANDROID_AVD_HOME="$QA_ROOT/avd"
export CARGO_HOME="$QA_ROOT/toolchains/cargo"
export RUSTUP_HOME="$QA_ROOT/toolchains/rustup"
export CARGO_BUILD_JOBS=${EASYGOT_BUILD_JOBS:-2}
export PATH="$CARGO_HOME/bin:$ANDROID_HOME/cmdline-tools/16.0/bin:$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$QA_ROOT/toolchains/gradle-8.11.1/bin:$JAVA_HOME/bin:$PATH"
mkdir -p "$ANDROID_HOME/cmdline-tools" "$ANDROID_AVD_HOME"
if [ ! -x "$ANDROID_HOME/cmdline-tools/16.0/bin/sdkmanager" ]; then
  curl -fL --retry 3 https://dl.google.com/android/repository/commandlinetools-linux-12266719_latest.zip -o "$QA_ROOT/downloads/android-tools.zip"
  unzip -tq "$QA_ROOT/downloads/android-tools.zip"
  unzip -q "$QA_ROOT/downloads/android-tools.zip" -d "$QA_ROOT/downloads/android-tools"
  mv "$QA_ROOT/downloads/android-tools/cmdline-tools" "$ANDROID_HOME/cmdline-tools/16.0"
fi
if [ ! -x "$QA_ROOT/toolchains/gradle-8.11.1/bin/gradle" ]; then
  curl -fL --retry 3 https://services.gradle.org/distributions/gradle-8.11.1-bin.zip -o "$QA_ROOT/downloads/gradle.zip"
  GRADLE_SHA=$(curl -fsSL https://services.gradle.org/distributions/gradle-8.11.1-bin.zip.sha256)
  printf '%s  %s\n' "$GRADLE_SHA" "$QA_ROOT/downloads/gradle.zip" | sha256sum -c -
  unzip -q "$QA_ROOT/downloads/gradle.zip" -d "$QA_ROOT/toolchains"
fi
if [ ! -x "$CARGO_HOME/bin/rustup" ]; then
  curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs -o "$QA_ROOT/downloads/rustup-init.sh"
  sh "$QA_ROOT/downloads/rustup-init.sh" -y --no-modify-path --profile minimal --default-toolchain 1.99.1
fi
rustup toolchain install 1.99.1 --profile minimal --component rustfmt
export RUSTUP_TOOLCHAIN=1.99.1
rustup target add aarch64-linux-android x86_64-linux-android
python3 -m venv "$QA_ROOT/toolchains/python"
export PATH="$QA_ROOT/toolchains/python/bin:$PATH"
python3 -m pip install 'fonttools==4.61.1' 'pillow==11.3.0'
python3 scripts/prepare_fonts.py
set +o pipefail
yes | sdkmanager --sdk_root="$ANDROID_HOME" --licenses > "$QA_ROOT/evidence/sdk-licenses.log" 2>&1
SDK_LICENSE_RESULT=${PIPESTATUS[1]}
set -o pipefail
test "$SDK_LICENSE_RESULT" = 0
sdkmanager --sdk_root="$ANDROID_HOME" 'platform-tools' 'emulator' 'platforms;android-35' 'build-tools;35.0.0' 'ndk;28.2.13676358' \
  'system-images;android-29;default;x86_64' 'system-images;android-35;default;x86_64'
{ rustc --version; cargo --version; java -version; gradle --version; emulator -version; sdkmanager --list_installed; } > "$QA_ROOT/evidence/toolchains.txt" 2>&1
cargo test --locked --lib
cargo check --locked --features ui-check
rustfmt --check --edition 2021 rust/src/controller.rs rust/src/ui.rs rust/src/practice.rs rust/src/platform.rs rust/src/media.rs build.rs examples/render_ui.rs
cargo install cargo-ndk --locked --version 3.5.4
rm -f app/src/main/jniLibs/arm64-v8a/libeasygot.so app/src/main/jniLibs/x86_64/libeasygot.so
cargo ndk -t arm64-v8a -p 26 -o app/src/main/jniLibs build --release
# Local VPS builds use their own TEST signing identity, not the published key.
# The actual downloadable app continues to be signed and checked by Actions.
gradle lintDebug assemblePreview --no-daemon
python3 scripts/check_apk_fonts.py app/build/outputs/apk/preview/app-preview.apk
mkdir -p "$QA_ROOT/evidence/arm64"
cp app/build/outputs/apk/preview/app-preview.* "$QA_ROOT/evidence/arm64/"
rm -f app/src/main/jniLibs/arm64-v8a/libeasygot.so
cargo ndk -t x86_64 -p 26 -o app/src/main/jniLibs build --release
gradle assembleDebug assemblePreview --no-daemon
python3 scripts/check_apk_fonts.py app/build/outputs/apk/debug/app-debug.apk app/build/outputs/apk/preview/app-preview.apk
ACCEL=off
if [ -r /dev/kvm ] && [ -w /dev/kvm ] && emulator -accel-check; then ACCEL=on; fi
printf '%s\n' "$ACCEL" > "$QA_ROOT/evidence/emulator-acceleration.txt"
export ANDROID_I_WANT_MY_TCG=yes
export ANDROID_SERIAL=emulator-5580
for API in 29 35; do
  # Respect the temporary server's shutdown window; never start another run late.
  TOKYO_HOUR=$(TZ=Asia/Tokyo date +%H)
  if [ "${TOKYO_HOUR#0}" -ge 21 ]; then echo 'Stopping before the temporary server shutdown window.'; exit 1; fi
  printf 'no\n' | avdmanager create avd --force --name "easygot-vps-$API" --package "system-images;android-$API;default;x86_64" --device pixel_5
  rm -rf qa-captures
  mkdir -p qa-captures
  cp app/build/outputs/apk/{debug,preview}/*.font-check.json qa-captures/
  emulator -avd "easygot-vps-$API" -port 5580 -no-window -noaudio -no-boot-anim -no-snapshot -gpu swiftshader_indirect -accel "$ACCEL" -memory 2048 -cores 2 > "$QA_ROOT/evidence/emulator-api$API.log" 2>&1 &
  EMULATOR_PID=$!
  READY=false
  for _ in $(seq 1 450); do
    kill -0 "$EMULATOR_PID" 2>/dev/null || break
    if [ "$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = 1 ]; then READY=true; break; fi
    sleep 2
  done
  if [ "$READY" != true ]; then echo "API $API did not boot within 15 minutes; acceleration=$ACCEL"; exit 1; fi
  adb shell settings put global window_animation_scale 0
  adb shell settings put global transition_animation_scale 0
  adb shell settings put global animator_duration_scale 0
  RESULT=0
  bash scripts/android-smoke.sh && python3 scripts/android-flow.py && bash scripts/android-preview-smoke.sh && python3 scripts/android-font-compat.py || RESULT=$?
  cp -R qa-captures "$QA_ROOT/evidence/api$API"
  adb emu kill || true
  wait "$EMULATOR_PID" || true
  EMULATOR_PID=''
  test "$RESULT" = 0
done
printf '%s\n' 'Both Android emulator suites passed.' > "$QA_ROOT/evidence/result.txt"
