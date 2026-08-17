pkgname=barbar
pkgver=0.1.0
pkgrel=1
pkgdesc="Another Wayland Bar"
arch=('x86_64')
url="https://github.com/BrynGhiffar/barbar"
depends=('gcc-libs')
makedepends=('cargo')
source=()
sha256sums=()

prepare() {
	cd "$startdir"
	export RUSTUP_TOOLCHAIN=stable
	cargo fetch --locked --target "$CARCH-unknown-linux-gnu"
}

build() {
	cd "$startdir"
	echo "RUSTC_VERSION: $(rustc --version)"
	echo "CARGO_VERSION: $(cargo --version)"
	export RUSTUP_TOOLCHAIN=stable
	export CARGO_TARGET_DIR=target
	cargo build --frozen --release --all-features
}

package() {
	cd "$startdir"
	install -Dm755 "target/release/barbar" "$pkgdir/usr/bin/barbar"
}
