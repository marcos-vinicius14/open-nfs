#!/usr/bin/env bash
# ==============================================================================
# Open-NFS Full APK Packaging Script
# Compiles ARM64 native library, converts game assets, packages, aligns, signs
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${REPO_ROOT}"

echo "=== Open-NFS: Packaging Full Standalone APK ==="

# 1. Detect Android SDK & NDK
ANDROID_HOME="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-/home/marcos/Android/Sdk}}"
if [ ! -d "${ANDROID_HOME}" ]; then
    echo "ERROR: Android SDK not found at ${ANDROID_HOME}." >&2
    exit 1
fi

BUILD_TOOLS_DIR=$(find "${ANDROID_HOME}/build-tools" -mindepth 1 -maxdepth 1 -type d | sort -V | tail -n 1)
PLATFORMS_DIR=$(find "${ANDROID_HOME}/platforms" -mindepth 1 -maxdepth 1 -type d -name "android-*" | sort -V | tail -n 1)
NDK_DIR="${ANDROID_NDK_HOME:-$(find "${ANDROID_HOME}/ndk" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | sort -V | tail -n 1)}"

AAPT2="${BUILD_TOOLS_DIR}/aapt2"
ZIPALIGN="${BUILD_TOOLS_DIR}/zipalign"
APKSIGNER="${BUILD_TOOLS_DIR}/apksigner"
ANDROID_JAR="${PLATFORMS_DIR}/android.jar"

if [ ! -x "${AAPT2}" ] || [ ! -x "${ZIPALIGN}" ] || [ ! -x "${APKSIGNER}" ] || [ ! -f "${ANDROID_JAR}" ]; then
    echo "ERROR: Required Android SDK build tools or android.jar missing." >&2
    exit 1
fi

if [ -z "${NDK_DIR}" ] || [ ! -d "${NDK_DIR}" ]; then
    echo "ERROR: Android NDK not found." >&2
    exit 1
fi

export ANDROID_NDK_HOME="${NDK_DIR}"
echo "Using Android SDK: ${ANDROID_HOME}"
echo "Using Build Tools: ${BUILD_TOOLS_DIR}"
echo "Using Platform:    ${PLATFORMS_DIR}"
echo "Using NDK:         ${NDK_DIR}"

# 2. Build Release ARM64 Native Library
echo "--> Compiling ARM64 release library (open-nfs-android-host)..."
cargo ndk -t aarch64-linux-android -P 26 build --release -p open-nfs-android-host

LIB_SO="target/aarch64-linux-android/release/libopen_nfs_android_host.so"
if [ ! -f "${LIB_SO}" ]; then
    echo "ERROR: Compiled library not found at ${LIB_SO}." >&2
    exit 1
fi

# 3. Setup Staging Directory
STAGING="target/android-staging"
rm -rf "${STAGING}"
mkdir -p "${STAGING}/lib/arm64-v8a"
mkdir -p "${STAGING}/assets/models"
mkdir -p "${STAGING}/assets/textures"

cp "${LIB_SO}" "${STAGING}/lib/arm64-v8a/libopen_nfs_android_host.so"

# 4. Prepare Assets (350Z Stock Car + Textures)
echo "--> Bundling converted runtime assets..."
if [ -f "game-assets/CARS/350Z/GEOMETRY.BIN" ]; then
    echo "    Converting CARS/350Z/GEOMETRY.BIN -> assets/models/350Z.onfs-model"
    cargo run -p open-nfs-converter -- -i game-assets/CARS/350Z/GEOMETRY.BIN -o "${STAGING}/assets/models/350Z.onfs-model"
fi

if [ -f "game-assets/CARS/350Z/TEXTURES.BIN" ]; then
    echo "    Copying CARS/350Z/TEXTURES.BIN -> assets/textures/350Z.bin"
    cp game-assets/CARS/350Z/TEXTURES.BIN "${STAGING}/assets/textures/350Z.bin"
fi

# 5. Compile Android Resources
echo "--> Compiling Android XML resources..."
"${AAPT2}" compile --dir crates/android-host/android/res -o "${STAGING}/compiled_res.zip"

# 6. Link APK with AAPT2
echo "--> Linking base APK..."
"${AAPT2}" link \
    -I "${ANDROID_JAR}" \
    --manifest crates/android-host/android/AndroidManifest.xml \
    -A "${STAGING}/assets" \
    -0 "bin" \
    -0 "onfs-model" \
    -o "${STAGING}/base.apk" \
    "${STAGING}/compiled_res.zip"


# 7. Add Uncompressed Native Library (.so) for Direct MMAP
echo "--> Adding uncompressed native ARM64 library..."
(cd "${STAGING}" && zip -u -0 base.apk lib/arm64-v8a/libopen_nfs_android_host.so)

# 8. Align APK (4-byte alignment, page-align .so)
echo "--> Aligning APK..."
"${ZIPALIGN}" -p -f 4 "${STAGING}/base.apk" "${STAGING}/aligned.apk"

# 9. Ensure Debug Keystore Exists
KEYSTORE="${HOME}/.android/debug.keystore"
if [ ! -f "${KEYSTORE}" ]; then
    echo "--> Generating debug keystore at ${KEYSTORE}..."
    mkdir -p "${HOME}/.android"
    keytool -genkey -v -keystore "${KEYSTORE}" -storepass android -alias androiddebugkey \
        -keypass android -keyalg RSA -keysize 2048 -validity 10000 -dname "CN=Android Debug,O=Android,C=US"
fi

# 10. Sign APK
OUTPUT_APK="target/open-nfs-full.apk"
echo "--> Signing APK -> ${OUTPUT_APK}..."
"${APKSIGNER}" sign \
    --ks "${KEYSTORE}" \
    --ks-pass pass:android \
    --in "${STAGING}/aligned.apk" \
    --out "${OUTPUT_APK}"

# 11. Verify Alignment & Signature
echo "--> Verifying package..."
"${ZIPALIGN}" -c -p 4 "${OUTPUT_APK}"
"${APKSIGNER}" verify "${OUTPUT_APK}"

APK_SIZE=$(du -h "${OUTPUT_APK}" | cut -f1)
echo "============================================================"
echo "SUCCESS! Standalone Full APK generated:"
echo "Path: ${OUTPUT_APK} (${APK_SIZE})"
echo "============================================================"
