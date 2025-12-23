# testaustime-zed

## installation

1. compile `testaustime-ls` using `cargo build --release -p testaustime-ls`
1. copy it to $PATH
1. install this extension using `zed: install dev extension` and then choose the folder with this README.md file

installation procedure will be improved once support for codeberg is added to `cargo-dist`
https://github.com/axodotdev/cargo-dist/issues/1781

## configuration

zed config.json
```jsonc
{
  "lsp": {
    "testaustime": {
      "settings": {
        "api_key": "",
        "api_base_url": "", // optional, include protocol if used
        "debug_logs": false, // optional
      },
    },
  },
}
```

## view debug_logs

the logs can be seen using `dev: open language server logs`
