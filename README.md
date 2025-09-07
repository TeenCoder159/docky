# Docky

Docky is a simple custom macos dock that can be configured with a file in .config/docky/apps.json, and you can use custom icons for your app, by using your own images and providing their respective path in the json config file. [See the example config](.config/docky/apps.json)
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
