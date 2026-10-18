case "$1" in
e)	vi -p .x
	;;
b)	cargo build --release
	cargo install wasm-bindgen-cli
	rustup target add wasm32-unknown-unknown
	./web/build.sh
	;;
gui)	python3 -m http.server -d web 8000   # then open http://localhost:8000
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
*)	./target/release/musiex generate -k G -l 3 --rests --triplets -o lesson1 --wav
	;;
esac
