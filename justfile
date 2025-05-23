bindings:
    cargo build --package wordbase-sys
    cargo run --bin uniffi-bindgen generate \
        --library target/debug/libwordbase.so \
        --language kotlin \
        --out-dir wordbase-android/app/build/generated/

build-aarch64 profile="debug":
    cross build \
        --target-dir target/cross \
        --target aarch64-linux-android \
        --package wordbase-sys \
        {{ if profile == "release" { "--release" } else { "" } }}
    mkdir -p wordbase-android/app/build/generated/lib/arm64-v8a/
    cp \
        target/cross/aarch64-linux-android/{{ profile }}/libwordbase.so \
        wordbase-android/app/build/generated/lib/arm64-v8a/
