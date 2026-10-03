$env.config.show_banner = false
alias sd = sudo


alias zp = zypper
alias zyp = zypper
alias zpr = zypper
alias zypp = zypper
alias dup = sudo zypper dup
alias up = sudo zypper up
alias ff = fastfetch
alias nf = neofetch
alias mm = master-menu

def ref [] {
    sudo zypper -vvv ref
    sudo chown -R ($env.USER):($env.USER) /var/cache/zypp
    up-zypp
}

let pat_path = ($"/tmp/pat_($env.USER)")

if not ($pat_path | path exists) {
    cp $"($env.HOME)/Documents/pat" $pat_path
}

$env.pat = (open --raw $pat_path | str trim)
