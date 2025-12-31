#!/bin/sh
#
# Check if the compiler thinks either binary could potentially panic
set -eu

if [ -t 1 ]; then
	COLOR_RED="\033[1;31m"
	COLOR_GREEN="\033[1;32m"
	COLOR_DEFAULT="\033[0m"
else
	COLOR_RED=""
	COLOR_GREEN=""
	COLOR_DEFAULT=""
fi

abort() {
	echo "${COLOR_RED}ERROR: ${@}${COLOR_DEFAULT}"
	exit 1
}

cargo build --release || abort "\`cargo build --release\` failed"
strings ./target/release/connate | grep -q "panic" && abort "compiler thinks ./src/connate can panic"
strings ./target/release/conctl | grep -q "panic" && abort "compiler thinks ./src/conctl can panic"

echo "${COLOR_GREEN}Checks passed: connate and conctl cannot panic${COLOR_DEFAULT}"
