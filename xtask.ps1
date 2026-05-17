# Aether X OS xtask Gateway (Windows)
# This script allows running xtask commands directly: .\xtask build

$args_str = $args -join " "
cargo run -p xtask -- $args
