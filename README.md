# Moveline.nvim

Moveline is a simple plugin for moving lines up and down. It's written in Rust using my library [nvim-utils](https://github.com/willothy/nvim-utils).

## Installation

Moveline can be installed using any Neovim package manager that supports build commands.

### Lazy.nvim

```lua
{
    'willothy/moveline.nvim'
    build = 'make'
}
```

### Packer.nvim

```lua
use('willothy/moveline', { run = 'make' })
```

## Usage

Moveline does not set any keybinds. It simply exports two functions: `move_line_up`, and `move_line_down`.

You can use these functions to set your own keybinds. Here's my setup:

```lua
local moveline = require('moveline')
vim.keymap.set('n', '<M-k>', moveline.move_line_up)
vim.keymap.set('n', '<M-j>', moveline.move_line_down)
```
