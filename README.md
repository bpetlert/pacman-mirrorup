# pacman-mirrorup

[![Release](https://img.shields.io/github/v/release/bpetlert/pacman-mirrorup?style=flat-square)](https://github.com/bpetlert/pacman-mirrorup/releases/latest)
[![AUR](https://img.shields.io/aur/version/pacman-mirrorup?style=flat-square)](https://aur.archlinux.org/packages/pacman-mirrorup)
[![License](https://img.shields.io/github/license/bpetlert/pacman-mirrorup?style=flat-square)](./COPYING)

**pacman-mirrorup** is a program to retrieve the best and latest Pacman mirror list based on user's geography.

**pacman-mirrorup** uses JSON from [Arch Linux's mirror status](https://archlinux.org/mirrors/status/) as data source.
It applies filters as the following to original data:

- Only active mirror
- HTTP/HTTPS protocol
- Completion at 100 percent
- Age under one hour

To take user's geography into consideration.
All filtered mirrors are tested data transfer rate.
The mirror's score from original data is weighted using transfer rate.
In other words, mirror's score is refined by using user's download speed.
Finally, the best _N_ mirrors are selected.
The best _N_ mirrors can be changed by `--mirror` option, the default is *10*.

The total number of synced mirrors to be tested is *100*.
This number can be changed by `--max-check` option.

To exclude a specific mirror, use `--exclude` option.
This command line option can be specified many times.
```bash
pacman-mirrorup \
    --exclude <A-DOMAIN-NAME> \
    --exclude domain=<A-DOMAIN-NAME> \
    --exclude country=<A-COUNTRY-NAME> \
    --exclude country_code=<A-COUNTRY-CODE>
```

The excluded mirror list could also be loaded from a file using `--exclude-from <FILE>` option.
Format is one excluded pattern per line.
Any pattern with prefix `!` is a negated pattern;
any matching mirror excluded by a previous pattern will become included again.
The excluded list from `--exclude` option take precedence over list from `--exclude-from` option.
The later occurrence of excluded pattern in list will also override any previous rules.
For example:

```ini
# /etc/pacman.d/excluded_mirrors

# This is a comment.
ban.this.mirror

; This is a comment too.
domain = ban.this-mirror.also

# Without `domain =`
ban.this.mirror

# Filter by country name
country = SomeCountry

# Filter by country code
country_code = SC

# Include this mirror, even though it was banned by previous rules
!domain = mirror.in.SomeCountry
```

## Installation

### Arch Linux

It is available on AUR as [pacman-mirrorup](https://aur.archlinux.org/packages/pacman-mirrorup).
To build and install arch package from GIT source:

```bash
git clone https://github.com/bpetlert/pacman-mirrorup.git
cd pacman-mirrorup
makepkg -p PKGBUILD.local
pacman -U pacman-mirrorup-xxxx-1-x86_64.pkg.tar
```

## Usage

### Run as command line

pacman-mirrorup can also be used directly from console.
Without `--output-file` option, pacman-mirrorup will display output on `STDOUT`.

### Run as systemd service

pacman-mirrorup provides systemd timer unit which can be enabled by

```bash
systemctl enable --now pacman-mirrorup.timer
```

To change the options of pacman-mirrorup service, run `systemctl edit pacman-mirrorup.service`

```ini
# /etc/systemd/system/pacman-mirrorup.service.d/override.conf

[Service]
Environment='MIRRORUP_ARGS=--output-file /etc/pacman.d/mirrorlist --threads 10'
```

To show log message higher than waning try set https://docs.rs/env_logger/latest/env_logger/#enabling-logging[`RUST_LOG`] environment variable. For example:

```ini
# /etc/systemd/system/pacman-mirrorup.service.d/override.conf

[Service]
Environment='RUST_LOG=pacman_mirrorup=debug'
Environment='MIRRORUP_ARGS=--output-file /etc/pacman.d/mirrorlist --threads 10'
```

To change the options of pacman-mirrorup timer, run `systemctl edit pacman-mirrorup.timer`

```ini
# /etc/systemd/system/pacman-mirrorup.timer.d/override.conf

[Timer]
OnCalendar=
OnCalendar=daily
```

## License

[GNU General Public License v3.0 or later](./COPYING)
