
android: 
	cargo ndk \
		-t arm64-v8a \
		-t armeabi-v7a \
		-t x86_64 \
		--platform 24 \
		-o kmp/composeApp/src/androidMain/jniLibs \
		build --release

CRATE_NAME := roxy-proxy
OUT_CRATE_NAME := roxy_proxy
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
		target/aarch64-apple-ios-sim/release/lib$(OUT_CRATE_NAME).a \
		target/x86_64-apple-ios/release/lib$(OUT_CRATE_NAME).a \
		-output $(SIM_UNI_DIR)/lib$(OUT_CRATE_NAME).a

	@rm -rf $(OUT_DIR) 
	@mkdir -p $(OUT_DIR)
	@xcodebuild -create-xcframework \
		-library target/aarch64-apple-ios/release/lib$(OUT_CRATE_NAME).a -headers $(HEADER_DIR) \
		-library $(SIM_UNI_DIR)/lib$(OUT_CRATE_NAME).a -headers $(HEADER_DIR) \
		-output $(OUT_DIR)/$(XC_NAME)

	@echo "✅ Built $(OUT_DIR)/$(XC_NAME)"

.PHONY: darwin-arm darwin-intel darwin-all darwin-universal clean

# Paths
JVMMAIN_DIR ?= kmp/composeApp/src/jvmMain
RES_DIR     := $(JVMMAIN_DIR)/resources
ARM_OUT     := $(RES_DIR)/darwin-aarch64
INTEL_OUT   := $(RES_DIR)/darwin-x86_64
UNIV_OUT    := $(RES_DIR)/darwin-universal

# Triples
ARM_TRIPLE   := aarch64-apple-darwin
INTEL_TRIPLE := x86_64-apple-darwin

# Built dylibs (as outputs)
ARM_LIB   := $(ARM_OUT)/libroxy_proxy.dylib
INTEL_LIB := $(INTEL_OUT)/libroxy_proxy.dylib
UNIV_LIB  := $(UNIV_OUT)/libroxy_proxy.dylib

darwin-arm:   $(ARM_LIB)
darwin-intel: $(INTEL_LIB)
darwin-all:   darwin-arm darwin-intel
darwin-universal: $(UNIV_LIB)

$(ARM_LIB):
	@rm -rf "$(ARM_OUT)"
	@cargo build --release --target $(ARM_TRIPLE)
	@mkdir -p "$(ARM_OUT)"
	@cp "target/$(ARM_TRIPLE)/release/libroxy_proxy.dylib" "$(ARM_OUT)/"
	@echo "✅ ARM_LIB dylib at: $(ARM_LIB)"

$(INTEL_LIB):
	@rm -rf "$(INTEL_OUT)"
	@cargo build --release --target $(INTEL_TRIPLE)
	@mkdir -p "$(INTEL_OUT)"
	@cp "target/$(INTEL_TRIPLE)/release/libroxy_proxy.dylib" "$(INTEL_OUT)/"
	@echo "✅ Intel dylib at: $(INTEL_LIB)"

$(UNIV_LIB): $(ARM_LIB) $(INTEL_LIB)
	@rm -rf "$(UNIV_OUT)"
	@mkdir -p "$(UNIV_OUT)"
	@lipo -create \
		"$(ARM_LIB)" \
		"$(INTEL_LIB)" \
		-output "$(UNIV_LIB)"
	@echo "✅ Universal dylib at: $(UNIV_LIB)"

clean:
	@rm -rf "$(ARM_OUT)" "$(INTEL_OUT)" "$(UNIV_OUT)"
