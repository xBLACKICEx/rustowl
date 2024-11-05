# RustOwl

## Quick Start



## Manually Build

### HTTP server

#### Prerequisite

- `rustup` installed
    - You can install `rustup` from [this link](https://rustup.rs/).

This has been tested on macOS Sonoma 14.6.1 with `rustup` 1.27.1.
Other dependencies are locked in the configuration files and will be installed automatically.

We have also tested this on Ubuntu 24.04.1 with rustup 1.27.1.

Additional dependencies may be required.
We have confirmed that running `apt install -y build-essential` is necessary on a freshly installed Ubuntu for linking.

#### Build & Run

```bash
cd rustowl-server
cargo run
```


### Visual Studio Code (VSCode) extension

#### Prerequisite

- VSCode installed
    - You can install VSCode from [this link](https://code.visualstudio.com/).
- Node.js installed
- `yarn` installed
    - You can install `yarn` by running `npm install -g yarn`.

This has been tested on macOS Sonoma 14.6.1 with Visual Studio Code 1.95.1, nodejs v20.16.0, and `yarn` 1.22.22.
Other dependencies are locked in the configuration files and will be installed automatically.

#### Build & Run

First, install the dependencies, then open VSCode.

```bash
cd rustowl-vscode
yarn install --frozen-locked
code .
```

Press the `F5` key in the open VSCode window.
A new VSCode window with the extension enabled will appear.
