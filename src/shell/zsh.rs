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
"#.trim().to_string()
}
