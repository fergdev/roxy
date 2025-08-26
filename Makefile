
android: 
	cargo ndk \
		-t arm64-v8a \
		-t armeabi-v7a \
		-t x86_64 \
		--platform 24 \
		-o kmp/composeApp/src/androidMain/jniLibs \
		build --release

CRATE_NAME := roxy-proxy
OUT_DIR := kmp/composeApp/src/iosMain/libs
HEADER_DIR := include
XC_NAME := Roxy.xcframework

IOS_TARGETS := aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
SIM_UNI_DIR := target/ios-sim-universal

.PHONY: ios clean

ios:
	@echo "=== Building iOS targets for $(CRATE_NAME) ==="
	@rustup target add $(IOS_TARGETS) >/dev/null 2>&1 || true

	@if ! command -v cbindgen >/dev/null 2>&1; then \
		echo "Installing cbindgen..."; \
		cargo install cbindgen --locked; \
	fi

	@mkdir -p $(HEADER_DIR)
	@cbindgen --crate $(CRATE_NAME) --output $(HEADER_DIR)/$(CRATE_NAME).h

	@cargo build -p $(CRATE_NAME) --release --target aarch64-apple-ios
	@cargo build -p $(CRATE_NAME) --release --target aarch64-apple-ios-sim
	@cargo build -p $(CRATE_NAME) --release --target x86_64-apple-ios

	@mkdir -p $(SIM_UNI_DIR)
	@lipo -create \
		target/aarch64-apple-ios-sim/release/lib$(CRATE_NAME).a \
		target/x86_64-apple-ios/release/lib$(CRATE_NAME).a \
		-output $(SIM_UNI_DIR)/lib$(CRATE_NAME).a

	@mkdir -p $(OUT_DIR)
	@xcodebuild -create-xcframework \
		-library target/aarch64-apple-ios/release/lib$(CRATE_NAME).a -headers $(HEADER_DIR) \
		-library $(SIM_UNI_DIR)/lib$(CRATE_NAME).a -headers $(HEADER_DIR) \
		-output $(OUT_DIR)/$(XC_NAME)

	@echo "✅ Built $(OUT_DIR)/$(XC_NAME)"

clean:
	@cargo clean -p $(CRATE_NAME)
	@rm -rf $(OUT_DIR)/$(XC_NAME) $(HEADER_DIR) $(SIM_UNI_DIR)
