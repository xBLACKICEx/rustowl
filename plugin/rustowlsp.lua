local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

vim.api.nvim_set_hl(0, 'lifetime', { undercurl = true, sp = '#00cc00' })
vim.api.nvim_set_hl(0, 'imm_borrow', { undercurl = true, sp = '#0000cc' })
vim.api.nvim_set_hl(0, 'mut_borrow', { undercurl = true, sp = '#cc00cc' })
vim.api.nvim_set_hl(0, 'move', { undercurl = true, sp = '#cccc00' })
vim.api.nvim_set_hl(0, 'call', { undercurl = true, sp = '#cccc00' })
vim.api.nvim_set_hl(0, 'outlive', { undercurl = true, sp = '#cc0000' })

if not configs.rustowl then
  configs.rustowl = {
    default_config = {
      cmd = { 'cargo', 'owlsp' },
      root_dir = lspconfig.util.root_pattern('Cargo.toml', '.git'),
      filetypes = { 'rust' },
      on_attach = function(_, _) end,
    },
  }
end
