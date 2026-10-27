#!/usr/bin/env sh
# Build an installable APK at android/app/build/outputs/apk/debug/app-debug.apk
# (debug-signed for sideloading; the Rust game itself is built in release mode).
#
# Needs: Android SDK (ANDROID_HOME) with NDK (ANDROID_NDK_HOME), JDK 17,
#        rustup target add aarch64-linux-android
#        cargo install cargo-ndk
#        Gradle 8.11+ on PATH, or a wrapper created with `gradle wrapper`.
set -eu
cd "$(dirname "$0")/.."

cargo ndk --platform 29 --target arm64-v8a \
  --output-dir android/app/src/main/jniLibs \
  build --release -p zrpg_android

if [ -x android/gradlew ]; then
  (cd android && ./gradlew assembleDebug)
else
  gradle -p android assembleDebug
fi

echo "APK: android/app/build/outputs/apk/debug/app-debug.apk"
echo "Install with: adb install -r android/app/build/outputs/apk/debug/app-debug.apk"
