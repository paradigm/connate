EAPI=8

CRATES=""

inherit cargo savedconfig

DESCRIPTION="Service manager for Linux"
HOMEPAGE="https://github.com/paradigm/connate"

if [[ ${PV} == 9999 ]]; then
	inherit git-r3
	EGIT_REPO_URI="https://github.com/paradigm/connate.git"
else
	SRC_URI="
		https://github.com/paradigm/connate/archive/v${PV}.tar.gz -> ${P}.tar.gz
		${CARGO_CRATE_URIS}
	"
	KEYWORDS="~amd64 ~x86"
fi

LICENSE="MIT"
SLOT="0"

IUSE="+host-checks +init +settle"

# No runtime dependencies - statically linked, no libc
# Block other init providers when installing as init
RDEPEND="
	init? (
		!sys-apps/sysvinit
		!sys-apps/systemd
	)
"
DEPEND=""
BDEPEND="|| ( dev-lang/rust-bin dev-lang/rust )"

# Binary is statically linked and stripped; ignore QA warnings
QA_FLAGS_IGNORED="
	sbin/connate
	usr/bin/connate
	usr/bin/conctl
"

src_unpack() {
	if [[ ${PV} == 9999 ]]; then
		git-r3_src_unpack
		cargo_live_src_unpack
	else
		cargo_src_unpack
	fi
}

src_prepare() {
	default

	restore_config src/config/config.rs
}

src_configure() {
	local myfeatures=(
		$(usev host-checks)
		$(usev settle)
	)
	cargo_src_configure --no-default-features
}

src_compile() {
	cargo_src_compile
}

src_install() {
	if use init; then
		# Install connate to /sbin for system-wide init usage
		into /
		dosbin "$(cargo_target_dir)/connate"
		dosym connate /sbin/init
	else
		# Install connate to /usr/bin for user session usage
		dobin "$(cargo_target_dir)/connate"
	fi

	# Install conctl control utility
	dobin "$(cargo_target_dir)/conctl"

	# Documentation
	einstalldocs
}

pkg_postinst() {
	savedconfig_pkg_postinst

	if [[ -z ${REPLACING_VERSIONS} ]]; then
		elog "Connate requires compile-time configuration."
		elog ""
		elog "To configure connate:"
		elog "  1. Edit the saved configuration file:"
		elog "     \${EDITOR} /etc/portage/savedconfig/${CATEGORY}/${PF}"
		elog "  2. Re-emerge the package to apply changes"
		elog ""
		elog "Configuration references:"
		elog "  - API: https://github.com/paradigm/connate/blob/master/src/config/config_api.rs"
		elog "  - Examples: https://github.com/paradigm/connate/tree/master/src/config/"
		elog ""
		if use init; then
			elog "Connate was installed as /sbin/init."
			elog "Ensure your bootloader is configured appropriately."
		else
			elog "Connate was installed to /usr/bin/connate for user sessions."
			elog "Use 'connate' to start and 'conctl' to control it."
		fi
	fi

	if use init; then
		ewarn "WARNING: Replacing your init system can render your system unbootable."
		ewarn "Ensure you have a rescue medium available before rebooting."
	fi
}
