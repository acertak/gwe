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
    local cur subcommand
    cur="${COMP_WORDS[COMP_CWORD]}"
    subcommand="${COMP_WORDS[1]}"
    local commands=(add list rm cd init shell-init config cursor wind anti claude codex gemini cli -e -c)

    if [ $COMP_CWORD -le 1 ]; then
        COMPREPLY=( $(compgen -W "${commands[*]}" -- "$cur") )
        return 0
    fi

    case "$subcommand" in
        cd|rm|cursor|wind|anti|claude|codex|gemini|cli|-e|-c)
            local rows needle repo_root show_list
            rows=$(command gwe list --completion 2>/dev/null) || return 0
            needle=$(printf '%s' "$cur" | tr '[:upper:]' '[:lower:]')
            COMPREPLY=()
            show_list=0
            if [ "${COMP_TYPE:-0}" -eq 63 ]; then
                show_list=1
            fi

            local -a display_rows
            while IFS=$'\t' read -r name branch abs_path; do
                [ -z "$name" ] && continue
                if [ "$subcommand" = "rm" ] && [ "$name" = "@" ]; then
                    continue
                fi
                if [ -z "$repo_root" ] && [ "$name" = "@" ]; then
                    repo_root="$abs_path"
                fi
                local haystack haystack_lower
                haystack="$name $branch $abs_path"
                haystack_lower=$(printf '%s' "$haystack" | tr '[:upper:]' '[:lower:]')
                if [ -z "$needle" ] || [[ "$haystack_lower" == *"$needle"* ]]; then
                    COMPREPLY+=("$name")
                    if [ $show_list -eq 1 ]; then
                        local rel_path
                        rel_path=$(_gwe_relpath "$repo_root" "$abs_path")
                        display_rows+=("$name	$branch	$rel_path")
                    fi
                fi
            done <<< "$rows"
            if [ $show_list -eq 1 ] && [ ${#display_rows[@]} -gt 0 ]; then
                printf '\n' >&2
                printf '%s\n' "${display_rows[@]}" >&2
                COMPREPLY=()
            fi
            return 0
            ;;
    esac
}

_gwe_relpath() {
    local base="$1"
    local target="$2"
    if [ -z "$base" ] || [ -z "$target" ]; then
        printf '%s' "$target"
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
    printf '%s' "$target"
}

complete -F _gwe_complete gwe
"#.trim().to_string()
}
