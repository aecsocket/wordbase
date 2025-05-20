bindings:
    cargo build --package wordbase-engine-sys
    cargo run --bin uniffi-bindgen generate \
        --library target/debug/libwordbase_engine.so \
        --language kotlin \
        --out-dir wordbase-android/app/build/generated/

build-aarch64:
    cross build --target aarch64-linux-android --package wordbase-engine-sys
    cp target/aarch64-linux-android/debug/libwordbase_engine.so wordbase-android/app/build/generated/lib/arm64-v8a/
