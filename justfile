bindings:
    cargo build --target x86_64-unknown-linux-gnu --package wordbase-engine-sys
    cargo run --bin uniffi-bindgen generate \
        --library target/x86_64-unknown-linux-gnu/debug/libwordbase.so \
        --language kotlin \
        --out-dir wordbase-android/app/build/generated/

build-aarch64:
    cross build --target aarch64-linux-android --package wordbase-engine-sys
    cp target/aarch64-linux-android/debug/libwordbase.so wordbase-android/app/build/generated/lib/arm64-v8a/
