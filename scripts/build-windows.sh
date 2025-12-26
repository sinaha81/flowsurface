#!/bin/bash
# اسکریپت ساخت نسخه ویندوز
EXE_NAME="flowsurface.exe"
ARCH=${1:-x86_64} # معماری سیستم: x86_64 یا aarch64
VERSION=$(grep '^version = ' Cargo.toml | cut -d'"' -f2) # استخراج نسخه از فایل Cargo.toml

# بروزرسانی نسخه پکیج در فایل Cargo.toml
cargo install cargo-edit
cargo set-version $VERSION

rustup override set stable-msvc

# تنظیم نوع هدف (Target Triple) و نام فایل فشرده
if [ "$ARCH" = "aarch64" ]; then
  TARGET_TRIPLE="aarch64-pc-windows-msvc"
  ZIP_NAME="flowsurface-aarch64-windows.zip"
else
  TARGET_TRIPLE="x86_64-pc-windows-msvc"
  ZIP_NAME="flowsurface-x86_64-windows.zip"
fi

# ساخت فایل اجرایی (Build)
rustup target add $TARGET_TRIPLE
cargo build --release --target=$TARGET_TRIPLE

# ایجاد پوشه موقت برای بسته‌بندی
mkdir -p target/release/win-portable

# کپی کردن فایل اجرایی و دارایی‌ها (Assets)
cp "target/$TARGET_TRIPLE/release/$EXE_NAME" target/release/win-portable/
if [ -d "assets" ]; then
    cp -r assets target/release/win-portable/
fi

# ایجاد فایل فشرده (Zip)
cd target/release
powershell -Command "Compress-Archive -Path win-portable\* -DestinationPath $ZIP_NAME -Force"
echo "فایل $ZIP_NAME ایجاد شد"