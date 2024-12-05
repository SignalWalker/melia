list-recipes:
	@just --list

set positional-arguments

# use cargo-watch to run `cargo run -- daemon` whenever a change is detected in this directory.
watch-run-daemon *args='':
	python3 ./bin/run-watch.py
