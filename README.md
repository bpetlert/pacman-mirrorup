# pacman-mirrorup

[![Release](https://img.shields.io/github/v/tag/bpetlert/pacman-mirrorup?include_prereleases&label=release&style=flat-square)](https://github.com/bpetlert/pacman-mirrorup/releases/latest)
[![AUR:
pacman-mirrorup](https://img.shields.io/aur/version/pacman-mirrorup?style=flat-square)](https://aur.archlinux.org/packages/pacman-mirrorup/)
[![License:
MIT](https://img.shields.io/github/license/bpetlert/pacman-mirrorup?style=flat-square)](./LICENSE)

A service to retrieve the best and latest Pacman mirror list based on
user's geography

Pacman-mirrorup uses JSON from [Arch Linux's mirror
status](https://www.archlinux.org/mirrors/status/) as data source. It
applies filters as the following to original data:

1.  Only active mirror
2.  HTTP/HTTPS protocol
3.  Completion at 100 percent
4.  Age under one hour

To take user's geography into consideration. All filtered mirrors are
tested data transfer rate. The mirror's score from original data is
weighted using transfer rate and then select the best N mirrors.

## Installation

### Arch Linux

It is available on AUR as
[pacman-mirrorup](https://aur.archlinux.org/packages/pacman-mirrorup/).
To build and install arch package from GIT source:

``` bash
$ git clone https://github.com/bpetlert/pacman-mirrorup.git
...
$ cd pacman-mirrorup
$ makepkg -p PKGBUILD.local
...
$ pacman -U pacman-mirrorup-xxxx-1-x86_64.pkg.tar
```

Then enable/start pacman-mirrorup.timer

``` bash
$ systemctl enable pacman-mirrorup.timer
...
$ systemctl start pacman-mirrorup.timer
```

## Configuration

To change the options of pacman-mirrorup service, run `systemctl edit
pacman-mirrorup.service`

``` ini
/etc/systemd/system/pacman-mirrorup.service.d/override.conf
-------------------------------------------------------------------------

[Service]
Environment='MIRRORUP_ARGS=-v --output-file /etc/pacman.d/mirrorlist --threads 20'
```

Supported options are:

  - `-m`, `--mirrors <mirrors>` Limit the list to the n mirrors with the
    highest score. \[default: 10\]
  - `-o`, `--output-file <output-file>` Mirror list output file
  - `-S`, `--source-url <source-url>` Arch Linux mirrors status's data
    source \[default: `https://www.archlinux.org/mirrors/status/json/`\]
  - `-s`, `--stats-file <stats-file>` Statistics output file
  - `-T`, `--threads <threads>` The maximum number of threads to use
    when measure transfer rate \[default: 5\]
  - `-v`, `--verbose` Increment verbosity level once per call. Default
    is showing error.
      - `-v`: warn
      - `-vv`: info
      - `-vvv`: debug
      - `-vvvv`: trace

To change the options of pacman-mirrorup timer, run `systemctl edit
pacman-mirrorup.timer`

``` ini
/etc/systemd/system/pacman-mirrorup.timer.d/override.conf
-------------------------------------------------------------------------

[Timer]
OnCalendar=daily
```

## License

**[MIT license](./LICENSE)**
