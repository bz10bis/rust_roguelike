
help:
	echo "make run    -run executable"

run:
	cd "$(EXE_DIR)/" \
	&& ./$(EXE_FILE)

crun:
	cargo run

cbuild:
	cargo build

