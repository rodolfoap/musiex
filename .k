git clean -xdf
case "$1" in
k)	rm -rfv /usr/local/bin/clipai* /home/rap/.local/bin/clipai # ~/.config/clipai/
	;;
esac
