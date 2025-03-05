CHARON=(which charon)

# 检查文件是否存在且具有可执行权限
if [ -x "$CHARON" ]; then
  echo "$CHARON exists, so skip installation"
else
  cd charon/charon
  # Install charon / charon-driver / generate-ml
  cargo install --path . --locked
  cd ../..
fi

# Install cargo-charon-rudra
cargo install --path . --locked

# Generate insertion_sort.ullbc
charon --ullbc --no-merge-goto-chains --no-cargo --input tests/panic_safety/insertion_sort.rs

# Analyze with rudra
cargo-charon-rudra --file insertion_sort.ullbc
