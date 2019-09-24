# mirrorup

A service to retrieve the latest Pacman mirror list

Mirrorup uses JSON from [Arch Linux's mirror status](https://www.archlinux.org/mirrors/status/) as data source. It applies filters as the following to original data:

1. Only active mirror
2. HTTP/HTTPS protocol
3. Completion at 100 percent
4. Age under one hour

To take user's geography into consideration. All filtered mirrors are tested data transfer rate. The mirror's score from original data is weighted using transfer rate and then select the best N mirrors.

## Installation

### Arch Linux

Build and install arch package from source:

```bash
$ git clone https://github.com/bpetlert/mirrorup.git
...
$ cd mirrorup
$ makepkg -p PKGBUILD.local
...
$ pacman -U mirrorup-xxxx-1-x86_64.pkg.tar
```

Then enable/start mirrorup.timer

```bash
$ systemctl enable mirrorup.timer
...
$ systemctl start mirrorup.timer
```

## Configuration

To change the options of mirrorup service, run `systemctl edit mirrorup.service`

```ini
/etc/systemd/system/mirrorup.service.d/override.conf
-------------------------------------------------------------------------

[Service]
Environment='MIRRORUP_ARGS=-v --output-file /etc/pacman.d/mirrorlist --threads 20'
```

Supported options are:

- `-m`, `--mirrors <mirrors>` Limit the list to the n mirrors with the highest score. [default: 10]
- `-o`, `--output-file <output-file>` Mirror list output file
- `-S`, `--source-url <source-url>` Arch Linux mirrors status's data source [default:
  `https://www.archlinux.org/mirrors/status/json/`]
- `-s`, `--stats-file <stats-file>` Statistics output file
- `-T`, `--threads <threads>` The maximum number of threads to use when measure transfer rate [default: 5]
- `-v`, `--verbose` Increment verbosity level once per call. Default
  is showing error.
  - `-v`: warn
  - `-vv`: info
  - `-vvv`: debug
  - `-vvvv`: trace

To change the options of mirrorup timer, run `systemctl edit mirrorup.timer`

```ini
/etc/systemd/system/mirrorup.timer.d/override.conf
-------------------------------------------------------------------------

[Timer]
OnCalendar=daily
```

## License

**[MIT license](./LICENSE)**
