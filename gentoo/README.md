# Gentoo Ebuilds for Connate

This directory contains Gentoo ebuilds for installing Connate.

### Why Two Packages?

- **connate**: The actual service manager package.

- **service-manager**: Gentoo's `@system` set requires `virtual/service-manager`,
  which by default only recognizes openrc, systemd, and s6-rc. This override adds
  `sys-apps/connate[init]` as a valid provider, allowing you to remove other init
  systems without breaking dependency resolution.

## Installation

### 1. Set Up a Local Overlay

If you don't already have one:

```bash
mkdir -p /var/db/repos/localrepo/{metadata,profiles}
echo 'localrepo' > /var/db/repos/localrepo/profiles/repo_name
echo 'masters = gentoo' > /var/db/repos/localrepo/metadata/layout.conf

cat >> /etc/portage/repos.conf/localrepo.conf <<EOF
[localrepo]
location = /var/db/repos/localrepo
EOF
```

### 2. Copy Ebuilds to Overlay

```bash
# Connate package
mkdir -p /var/db/repos/localrepo/sys-apps/connate
cp connate-9999.ebuild /var/db/repos/localrepo/sys-apps/connate/

# Virtual override
mkdir -p /var/db/repos/localrepo/virtual/service-manager
cp service-manager-2.ebuild /var/db/repos/localrepo/virtual/service-manager/
```

### 3. Generate Manifests

```bash
cd /var/db/repos/localrepo/sys-apps/connate
ebuild connate-9999.ebuild manifest

cd /var/db/repos/localrepo/virtual/service-manager
ebuild service-manager-2.ebuild manifest
```

### 4. Accept Keywords

Live ebuilds have no keywords. Add to `/etc/portage/package.accept_keywords/connate`:

```
sys-apps/connate **
```

### 5. Configure

Reference:

- `../src/config/config_api.rs`
- `../src/config/example*.rs`

then create a connate config file at

```
/etc/portage/savedconfig/sys-apps/connate-9999
```

### 6. Install

As init:

```bash
emerge -av virtual/service-manager sys-apps/connate
```

As non-init:


```bash
echo "sys-apps/connate -init" >> /etc/portage/package.use/connate
emerge -av sys-apps/connate
```

The first build will probably fail due to lack of configuration.

## USE Flags

| Flag          | Default | Description                                                |
+---------------+---------+------------------------------------------------------------+
| `host-checks` | Yes     | Build-time path validation. Disable when cross-compiling.  |
| `init`        | Yes     | Install as `/sbin/init`. Conflicts with sysvinit/systemd.  |
| `settle`      | Yes     | Enable settle pipes for `conctl` to wait on state changes. |

### Initial Setup

1. Install connate (saves default config)
2. Edit the saved configuration:
   ```bash
   $EDITOR /etc/portage/savedconfig/sys-apps/connate-9999
   ```
3. Re-emerge to apply:
   ```bash
   emerge connate
   ```

### Configuration References

- [config_api.rs](../src/config/config_api.rs) - API documentation
- [config.rs](../src/config/config.rs) - Default configuration

## Updating

To rebuild with the latest upstream:

```bash
emerge --oneshot connate
```

To rebuild with config changes only (skip git fetch):

```bash
EGIT_OFFLINE=1 emerge --oneshot connate
```

## Replacing Your Init System

**WARNING**: Replacing your init system can render your system unbootable.
Ensure you have a rescue medium available before rebooting.

1. Ensure `USE=init` is set (default)
2. Install connate - this will block sysvinit/systemd
3. Verify `/sbin/init` points to connate
4. Reboot with rescue medium accessible
