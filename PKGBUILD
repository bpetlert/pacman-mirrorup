# Maintainer: Bhanupong Petchlert <bpetlert@gmail.com>
pkgname=mirrorup
pkgver=0.1.0
pkgrel=1
pkgdesc="A service to retrieve the latest Pacman mirror list"
arch=('x86_64')
url="https://github.com/bpetlert/mirrorup"
license=('MIT')
depends=('systemd')
makedepends=('rust' 'cargo')
provides=("${pkgname}")
conflicts=("${pkgname}")

# Build from local directory
source=()

# Using the most recent annotated tag reachable from the last commit.
pkgver() {
  cd "$startdir"
  git describe --long | sed 's/\([^-]*-g\)/r\1/;s/-/./g'
}

build() {
  cd "$startdir"

  # Ignore target_dir in ~/.cargo/config, use BUILDDIR from makepkg instead
  CARGO_TARGET_DIR="$srcdir/../target" cargo build --release --locked
}

package() {
  cd "$srcdir/../"
  install -Dm755 "target/release/mirrorup" "$pkgdir/usr/bin/mirrorup"

  install -Dm644 "$startdir/mirrorup.service" "$pkgdir/usr/lib/systemd/system/mirrorup.service"
  install -Dm644 "$startdir/mirrorup.timer" "$pkgdir/usr/lib/systemd/system/mirrorup.timer"

  install -Dm644 "$startdir/README.md" "$pkgdir/usr/share/doc/${pkgname}/README.md"
  install -Dm644 "$startdir/LICENSE" "$pkgdir/usr/share/licenses/${pkgname}/LICENSE"
}
