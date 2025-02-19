local M = {}

local state = { show = false }

---@class RustOwlHoverOptions
---@field enabled? boolean
---@field idle_time? number Time to hover with the cursor before triggering RustOwl

---@class RustOwlTriggerOptions
---@field hover? RustOwlHoverOptions Trigger RustOwl when hovering over a symbol with the cursor

---@class RustOwlOptions
---@field client? vim.lsp.ClientConfig | {} LSP client configuration
---@field trigger? RustOwlTriggerOptions
local options = {
  client = {},
  trigger = {
    hover = {
      enabled = true,
      idle_time = 2000,
    },
  },
}

---@return RustOwlOptions
function M.get_options()
  return options
end

---@param bufnr? number
function M.toggle(bufnr)
  local action = state.show and M.hide or M.show
  action(bufnr)
end

M.enable = require('rustowl.show-on-hover').enable

M.disable = require('rustowl.show-on-hover').disable

M.toggle = require('rustowl.show-on-hover').toggle

---@param opts? RustOwlOptions
function M.setup(opts)
  ---@type RustOwlOptions
  options = vim.tbl_deep_extend('keep', opts or {}, options)
  require('lspconfig').rustowlsp.setup(options.client)

  if options.trigger.hover.enabled then
    require('rustowl.show-on-hover').create_lsp_attach_autocmd()
  end
end

return M
