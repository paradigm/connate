# Copyright 2025 Gentoo Authors
# Distributed under the terms of the GNU General Public License v2

EAPI=8

DESCRIPTION="Virtual for service managers"
SLOT="0"
KEYWORDS="amd64 arm arm64 ~loong ppc ppc64 ~riscv x86"

RDEPEND="
	|| (
		sys-apps/connate[init]
		sys-apps/openrc
		sys-apps/s6-rc
		sys-apps/systemd
	)
"
