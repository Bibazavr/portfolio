#!/usr/bin/env bash
# Builds the site into dist/.
#
#   ./build.sh                    both plaza builds, then the classic page and the shell
#   ./build.sh plaza <renderer>   one plaza build: webgl2 or webgpu
#   ./build.sh site               the classic page and the shell, around existing plaza builds
set -euo pipefail
cd "$(dirname "$0")"

plaza() {
	# Keeps the machine's paths (home folder, checkout) out of the shipped binary.
	local registry="${CARGO_HOME:-$HOME/.cargo}/registry/src"
	local std_source
	std_source="$(rustc --print sysroot)/lib/rustlib/src/rust"
	(
		export RUSTFLAGS="--remap-path-prefix=$registry= --remap-path-prefix=$std_source=rust --remap-path-prefix=$PWD="
		cd crates/plaza
		trunk build --release --no-default-features --features "$1" --dist "../../dist/plaza/$1"
	)
}

site() {
	cargo run --locked -p classic -- dist
}

case "${1:-all}" in
	plaza) plaza "$2" ;;
	site) site ;;
	all)
		plaza webgl2
		plaza webgpu
		site
		;;
	*)
		echo "usage: $0 [plaza webgl2|webgpu | site]" >&2
		exit 1
		;;
esac
