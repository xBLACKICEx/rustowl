local M = {}

local state = { show = false }
local hl_ns = vim.api.nvim_create_namespace('rustowl')

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

---@param bufnr? number
function M.show(bufnr)
  local util = require('lspconfig.util')

  bufnr = util.validate_bufnr(bufnr or 0)
  local clients = util.get_lsp_clients { bufnr = bufnr, name = 'rustowlsp' }
  for _, client in ipairs(clients) do
    local line, col = unpack(vim.api.nvim_win_get_cursor(0))
    client:request(
      'rustowl/cursor',
      {
        position = { line = line - 1, character = col },
        document = vim.lsp.util.make_text_document_params(),
      },
      function(_, result, _)
        if result ~= nil then
          for _, deco in ipairs(result['decorations']) do
            local start = { deco['range']['start']['line'], deco['range']['start']['character'] }
            local finish = { deco['range']['end']['line'], deco['range']['end']['character'] }
            vim.highlight.range(
              bufnr,
              hl_ns,
              deco['type'],
              start,
              finish,
              { regtype = 'v', inclusive = true }
            )
          end
        end
      end,
      bufnr
    )
  end

  state.show = true
end

---@param bufnr? number
function M.hide(bufnr)
  vim.api.nvim_buf_clear_namespace(bufnr or 0, hl_ns, 0, -1)
  state.show = false
end

---@param bufnr? number
function M.toggle(bufnr)
  local action = state.show and M.hide or M.show
  action(bufnr)
end

---@param opts? RustOwlOptions
function M.setup(opts)
  ---@type RustOwlOptions
  options = vim.tbl_deep_extend('keep', opts or {}, options)
  require('lspconfig').rustowlsp.setup(options.client)

  if options.trigger.hover.enabled then
    local idle_time = options.trigger.hover.idle_time
    require('rustowl.hover').create_lsp_attach_autocmd(idle_time)
  end
end

return M
