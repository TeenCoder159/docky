# dock

Created with Create GPUI App.

- [`gpui`](https://www.gpui.rs/)
- [GPUI documentation](https://github.com/zed-industries/zed/tree/main/crates/gpui/docs)
- [GPUI examples](https://github.com/zed-industries/zed/tree/main/crates/gpui/examples)

<<<<<<< Updated upstream
## Usage

- Ensure Rust is installed - [Rustup](https://rustup.rs/)
- Run your app with `cargo run`
=======
## Usage:
I am currently using tmux to run it, but you can use nohup.

My Config is visible inside the .config, and you will need to modify the width of your dock if you do not have 6 apps (its in the main function and render function)

### Using Nohup:
```zsh
nohup cargo run --release &
```

### Using tmux (requires tmux):
```zsh
tmux
```
or enter an existing tmux session
then:
```
cargo run --release &
```
or
```zsh
cargo run --release
```



>>>>>>> Stashed changes
