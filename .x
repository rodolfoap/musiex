case "$1" in
e)	vi -p .x
	;;
merge)	git checkout main
	git fetch; git pull
	git checkout wip;
	git merge main;
	git push;
	;;
t)	cargo test
	cargo clippy --all-targets -- -D warnings
	;;
*)	cargo build --release
	./target/release/musiex generate -k G -l 3 --rests --triplets -o lesson1 --wav
	;;
esac
