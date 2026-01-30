pub fn script() -> String {
    r#"
gwe() {
    if [ "$1" = "cd" ]; then
        local dest
        dest=$(command gwe cd "${@:2}")
        local ret=$?
        if [ $ret -eq 0 ]; then
            cd "$dest"
        else
            return $ret
        fi
    else
        command gwe "$@"
    fi
}

_gwe_complete() {
    local -a commands
    commands=(add list rm cd init shell-init config cursor wind anti claude codex gemini cli -e -c)

    if (( CURRENT == 2 )); then
        compadd -a commands
        return
    fi

    local subcommand=${words[2]}
    case "$subcommand" in
        cd|rm|cursor|wind|anti|claude|codex|gemini|cli|-e|-c)
            local rows needle
            rows=$(command gwe list --completion 2>/dev/null) || return
            needle=${words[CURRENT]}
            needle=${needle:l}

            local -a names descs
            local repo_root=""
            local rel_path=""
            while IFS=$'\t' read -r name branch abs_path; do
                [[ -z $name ]] && continue
                if [[ $subcommand == rm && $name == @ ]]; then
                    continue
                fi
                if [[ -z $repo_root && $name == @ ]]; then
                    repo_root=$abs_path
                fi
                local haystack="${name} ${branch} ${abs_path}"
                local haystack_lower=${haystack:l}
                if [[ -z $needle || $haystack_lower == *${needle}* ]]; then
                    names+=("$name")
                    rel_path=$( _gwe_relpath "$repo_root" "$abs_path" )
                    descs+=("${branch} ${rel_path}")
                fi
            done <<< "$rows"

            if (( ${#names[@]} > 0 )); then
                compadd -d descs -- $names
            fi
            return
            ;;
    esac
}

_gwe_relpath() {
    local base="$1"
    local target="$2"
    if [[ -z $base || -z $target ]]; then
        echo "$target"
        return
    fi
    if command -v python3 >/dev/null 2>&1; then
        python3 - <<'PY' "$base" "$target"
import os,sys
print(os.path.relpath(sys.argv[2], sys.argv[1]), end="")
PY
        return
    fi
    if command -v perl >/dev/null 2>&1; then
        perl -e 'use File::Spec; print File::Spec->abs2rel($ARGV[1], $ARGV[0]);' "$base" "$target"
        return
    fi
    echo "$target"
}

compdef _gwe_complete gwe
"#.trim().to_string()
}
