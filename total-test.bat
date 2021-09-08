
cargo clean
cargo run -p app
cargo run -p app -- --compile

cargo clean
cargo run -p app --release
cargo run -p app --release -- --compile

