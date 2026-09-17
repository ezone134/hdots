#!/usr/bin/env bash

 symlink_function () {
	[ "$(readlink -f $1 2>/dev/null)" = "$(readlink -f $2 2>/dev/null)" ] || {
	[ -L $2 ] && {
	# rm $2 2>/dev/null
	sudo mv $2 /tmp/moved_deleted.${rnd}
	}
	sudo mv $2 $2.bak.$rnd 2>/dev/null
	sudo ln -s $1 $2  2>/dev/null
 }
 }
 symlink_function /tmp/hypr_global/99-power-source.rules /etc/udev/rules.d/99-power-source.rules
