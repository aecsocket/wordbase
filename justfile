generate-bindings:
    cargo build --release --package wordbase-sys
    mkdir -p wordbase-android/app/build/generated/lib/x86_64
    cp target/release/libwordbase.so wordbase-android/app/build/generated/lib/x86_64
    cargo run --bin uniffi-bindgen \
        generate \
        --library target/release/libwordbase.so \
        --language kotlin \
        --out-dir wordbase-android/app/build/generated/
