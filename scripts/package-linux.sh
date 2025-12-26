#!/bin/bash
# اسکریپت بسته‌بندی نسخه لینوکس
ARCH=${2:-x86_64}  # اگر آرگومان اول "package" باشد، آرگومان دوم معماری را مشخص می‌کند
TARGET="flowsurface"
PROFILE="release"
RELEASE_DIR="target/$PROFILE"
ARCHIVE_DIR="$RELEASE_DIR/archive"

# تنظیم نوع هدف (Target Triple) و نام فایل فشرده بر اساس معماری
if [ "$ARCH" = "aarch64" ]; then
  TRIPLE="aarch64-unknown-linux-gnu"
  ARCHIVE_NAME="$TARGET-aarch64-linux.tar.gz"
else
  TRIPLE="x86_64-unknown-linux-gnu"
  ARCHIVE_NAME="$TARGET-x86_64-linux.tar.gz"
fi

ARCHIVE_PATH="$RELEASE_DIR/$ARCHIVE_NAME"
BINARY="target/$TRIPLE/$PROFILE/$TARGET"

# تابع ساخت فایل اجرایی (Build)
build() {
  rustup target add $TRIPLE
  cargo build --release --target="$TRIPLE"
}

# تابع نمایش نام فایل فشرده
archive_name() {
  echo $ARCHIVE_NAME
}

# تابع نمایش مسیر فایل فشرده
archive_path() {
  echo $ARCHIVE_PATH
}

# تابع اصلی بسته‌بندی
package() {
  build
  mkdir -p "$ARCHIVE_DIR/bin"
  # نصب فایل اجرایی در پوشه مقصد با دسترسی‌های لازم
  install -Dm755 "$BINARY" -t "$ARCHIVE_DIR/bin"
  # کپی کردن دارایی‌ها (Assets) در صورت وجود
  if [ -d "assets" ]; then
    cp -r assets "$ARCHIVE_DIR/"
  fi
  # ایجاد فایل فشرده tar.gz
  tar czvf "$ARCHIVE_PATH" -C "$ARCHIVE_DIR" .
  echo "فایل فشرده در مسیر زیر ایجاد شد: $ARCHIVE_PATH"
}

# مدیریت دستورات ورودی
case "$1" in
  "package") package;;
  "archive_name") archive_name;;
  "archive_path") archive_path;;
  *)
    echo "دستورات موجود: package, archive_name, archive_path"
    ;;
esac