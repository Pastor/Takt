#!/usr/bin/env bash
# Очистка выделенного каталога сборки после завершения всех потребителей.
# Скрипты передают только каталоги, которыми управляют сами.
prune_build_profile() (
    local profile="$1"
    local entry
    [[ -d "$profile" && "$(cd -P -- "$profile" && pwd -P)" == "$profile" ]] || return 0
    shopt -s dotglob nullglob
    for entry in "$profile"/*; do
        if [[ -f "$entry" && ! -L "$entry" ]]; then
            case "$entry" in
                *.wasm) continue ;;
            esac
            [[ ! -x "$entry" ]] || continue
        fi
        rm -rf -- "$entry" || return 1
    done
)

prune_cargo_target() (
    local target="$1"
    local profile
    [[ -d "$target" && "$(cd -P -- "$target" && pwd -P)" == "$target" ]] || return 0
    for profile in debug release "${TAKT_WASM_PROFILE:-wasm}" wasm; do
        [[ "$profile" =~ ^[a-zA-Z0-9_-]+$ ]] || return 1
        prune_build_profile "$target/$profile" || return 1
        if [[ ! -L "$target/wasm32-unknown-unknown" ]]; then
            prune_build_profile "$target/wasm32-unknown-unknown/$profile" || return 1
        fi
    done
    if [[ ! -L "$target/wasm32-unknown-unknown" ]]; then
        rm -rf -- "$target/wasm32-unknown-unknown/tmp" || return 1
    fi
    rm -rf -- "$target/tmp" || return 1
    rm -f -- "$target/.rustc_info.json" "$target/CACHEDIR.TAG"
)

build_artifacts_finish() {
    local status=$?
    local cleanup_status=0
    trap - EXIT INT TERM
    prune_cargo_target "$BUILD_ARTIFACT_TARGET" || cleanup_status=$?
    local profile
    for profile in ${BUILD_ARTIFACT_PROFILES[@]+"${BUILD_ARTIFACT_PROFILES[@]}"}; do
        prune_build_profile "$profile" || cleanup_status=$?
    done
    rmdir -- "$BUILD_ARTIFACT_LOCK" || cleanup_status=$?
    if (( status == 0 )); then
        status=$cleanup_status
    fi
    exit "$status"
}

build_artifacts_start() {
    local target="$1"
    shift
    local parent
    parent="$(dirname "$target")"
    if [[ -L "$parent" || -L "$target" ]]; then
        echo "Каталог сборки и его родитель должны быть обычными каталогами: $target" >&2
        return 1
    fi
    mkdir -p -- "$parent" || return 1
    if [[ "$(cd -P -- "$parent" && pwd -P)" != "$parent" ]]; then
        echo "Каталог сборки расположен за символической ссылкой: $target" >&2
        return 1
    fi
    BUILD_ARTIFACT_TARGET="$target"
    BUILD_ARTIFACT_PROFILES=("$@")
    BUILD_ARTIFACT_LOCK="$target.lock"
    if ! mkdir -- "$BUILD_ARTIFACT_LOCK"; then
        echo "Каталог сборки занят: $target" >&2
        return 1
    fi
    trap build_artifacts_finish EXIT
    trap 'exit 130' INT
    trap 'exit 143' TERM
}
