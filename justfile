bindings:
    cargo build --package wordbase-sys
    cargo run --release --bin uniffi-bindgen generate \
        --library target/debug/libwordbase.so \
        --language kotlin \
        --out-dir wordbase-android/app/build/generated/

android-lib rust_target android_target profile="debug":
    cross build \
        --target-dir "target/cross/{{ rust_target }}" \
        --target "{{ rust_target }}" \
        --package wordbase-sys \
        {{ if profile == "release" { "--release" } else { "" } }}
    mkdir -p "wordbase-android/app/build/generated/lib/{{ android_target }}/"
    cp \
        "target/cross/{{ rust_target }}/{{ rust_target }}/{{ profile }}/libwordbase.so" \
        "wordbase-android/app/build/generated/lib/{{ android_target }}/"

android-lib-armv7 profile="debug": (android-lib "armv7-linux-androideabi" "armeabi-v7a" profile)
android-lib-aarch64 profile="debug": (android-lib "aarch64-linux-android" "arm64-v8a" profile)
android-lib-x86 profile="debug": (android-lib "i686-linux-android" "x86" profile)
android-lib-x86_64 profile="debug": (android-lib "x86_64-linux-android" "x86_64" profile)
android-libs profile="debug":
    echo '\
    layout { \
        pane split_direction="vertical" { \
            pane split_direction="horizontal" { \
                pane name="armv7" command="just" { \
                    args "android-lib-armv7" "{{ profile }}"; \
                }; \
                pane name="aarch64" command="just" { \
                    args "android-lib-aarch64" "{{ profile }}"; \
                }; \
            }; \
            pane split_direction="horizontal" { \
                pane name="x86" command="just" { \
                    args "android-lib-x86" "{{ profile }}"; \
                }; \
                pane name="x86_64" command="just" { \
                    args "android-lib-x86_64" "{{ profile }}"; \
                }; \
            }; \
        }; \
    } \
    ' > /tmp/wordbase-zellij-run.kdl
    -zellij delete-session wordbase
    zellij --new-session-with-layout /tmp/wordbase-zellij-run.kdl
