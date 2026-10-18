# ==============================================================================
# Open-NFS Engine — Master Makefile
# ==============================================================================

.PHONY: all help apk install run-android logcat desktop test clippy fmt clean

# Default target
all: help

help:
	@echo "========================================================================"
	@echo "                      Open-NFS Build System"
	@echo "========================================================================"
	@echo "  make apk          - Build and package target/open-nfs-full.apk"
	@echo "  make run-android  - Build, install and launch on connected Android phone"
	@echo "  make install      - Install target/open-nfs-full.apk via ADB"
	@echo "  make logcat       - Stream real-time Android device logs (filtered)"
	@echo "  make desktop      - Run desktop interactive vehicle viewer"
	@echo "  make test         - Run entire workspace test suite"
	@echo "  make clippy       - Run workspace linter checks"
	@echo "  make fmt          - Format all Rust code"
	@echo "  make clean        - Remove build artifacts and staging folders"
	@echo "========================================================================"

# Build and package the self-contained Full APK
apk:
	@./tools/package-apk.sh

# Install APK onto connected Android device
install: apk
	@echo "--> Checking connected ADB devices..."
	@adb get-state > /dev/null 2>&1 || (echo "ERROR: No Android device detected. Connect phone via USB and enable USB Debugging." && exit 1)
	@echo "--> Installing target/open-nfs-full.apk..."
	@adb install -r target/open-nfs-full.apk
	@echo "✓ APK installed successfully."

# Install and launch activity directly on device
run-android: install
	@echo "--> Launching Open-NFS on device..."
	@adb shell am start -n com.opennfs.engine/android.app.NativeActivity
	@echo "✓ Open-NFS launched. Use 'make logcat' to view logs."

# Stream real-time filtered device logs
logcat:
	@adb logcat -c
	@adb logcat -s OpenNFS:V NativeActivity:V AndroidRuntime:E DEBUG:E RustStdoutStderr:V TRACE:V

# Run desktop interactive viewer
desktop:
	@cargo run -p open-nfs-android-host --bin desktop-viewer

# Run all test gates
test:
	@cargo test --workspace

# Run Clippy
clippy:
	@cargo clippy --workspace --all-targets --all-features -- -D warnings

# Format code
fmt:
	@cargo fmt --all

# Clean staging and build
clean:
	@cargo clean
	@rm -rf target/android-staging target/open-nfs-full.apk
