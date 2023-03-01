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

Moveline does not set any keybinds. It simply exports four functions: `up` and `down` for moving single lines, and `block_up` and `block_down` for moving visual selections.

You can use these functions to set your own keybinds. Here's my setup:

```lua
local moveline = require('moveline')
vim.keymap.set('n', '<M-k>', moveline.up)
vim.keymap.set('n', '<M-j>', moveline.down)
vim.keymap.set('v', '<M-k>', moveline.block_up)
vim.keymap.set('v', '<M-j>', moveline.block_down)
```

Moveline functions accept counts. For example, with the keybinds above, typing `5<M-k>` will move 
the current line up 5 lines.
