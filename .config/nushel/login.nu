if not ("/tmp/auto_" + $env.USER | path exists) {
    touch ("/tmp/auto_" + $env.USER)
    hyprland
}