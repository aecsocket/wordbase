generate-bindings:
    cargo build --release --package wordbase-sys
    cargo run --bin uniffi-bindgen \
        generate \
        --library target/release/libwordbase.so \
        --language kotlin \
        --out-dir wordbase-android/app/build/generated/
