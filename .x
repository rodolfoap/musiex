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
*)	cargo run --release -p musiex-cli -- generate --help
	# 32 bars in G major, eighth notes, with rests and triplets
	cargo run --release -p musiex-cli -- generate -k G -l 3 --rests --triplets -o lesson1
	# 6/8 in the bass clef, reproducible
	cargo run --release -p musiex-cli -- generate -m 6/8 -c bass -k Dm -l 3 --ties -s 1234
	;;
esac
