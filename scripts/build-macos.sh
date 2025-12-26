#!/bin/bash
# اسکریپت ساخت نسخه مک‌او‌اس (macOS)
TARGET="flowsurface"
VERSION=$(grep '^version = ' Cargo.toml | cut -d'"' -f2) # استخراج نسخه از فایل Cargo.toml
ARCH=${1:-universal} # معماری سیستم: x86_64 یا aarch64 یا universal (هر دو)
RELEASE_DIR="target/release"

# تنظیم حداقل نسخه مک‌او‌اس مورد نیاز
export MACOSX_DEPLOYMENT_TARGET="11.0"

# افزودن اهداف ساخت برای اینتل و اپل سیلیکون
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin

mkdir -p "$RELEASE_DIR"

# ساخت برای معماری x86_64 (اینتل)
if [ "$ARCH" = "x86_64" ]; then
  cargo build --release --target=x86_64-apple-darwin
  cp "target/x86_64-apple-darwin/release/$TARGET" "$RELEASE_DIR/$TARGET"
  tar -czf "$RELEASE_DIR/${TARGET}-x86_64-macos.tar.gz" -C "$RELEASE_DIR" "$TARGET"
  echo "فایل $RELEASE_DIR/${TARGET}-x86_64-macos.tar.gz ایجاد شد"
  exit 0
fi

# ساخت برای معماری aarch64 (اپل سیلیکون)
if [ "$ARCH" = "aarch64" ]; then
  cargo build --release --target=aarch64-apple-darwin
  cp "target/aarch64-apple-darwin/release/$TARGET" "$RELEASE_DIR/$TARGET"
  tar -czf "$RELEASE_DIR/${TARGET}-aarch64-macos.tar.gz" -C "$RELEASE_DIR" "$TARGET"
  echo "فایل $RELEASE_DIR/${TARGET}-aarch64-macos.tar.gz ایجاد شد"
  exit 0
fi

# حالت پیش‌فرض: ساخت برای هر دو معماری و ایجاد فایل Universal
cargo build --release --target=x86_64-apple-darwin
cargo build --release --target=aarch64-apple-darwin

# ترکیب دو فایل اجرایی در یک فایل Universal با استفاده از lipo
lipo "target/x86_64-apple-darwin/release/$TARGET" "target/aarch64-apple-darwin/release/$TARGET" -create -output "$RELEASE_DIR/$TARGET"
tar -czf "$RELEASE_DIR/${TARGET}-universal-macos.tar.gz" -C "$RELEASE_DIR" "$TARGET"
echo "فایل $RELEASE_DIR/${TARGET}-universal-macos.tar.gz ایجاد شد"