local M = {}

local hl_ns = vim.api.nvim_create_namespace('rustowl')

---@param bufnr? number
function M.highlight(bufnr, line, col)
  local util = require('lspconfig.util')

  bufnr = util.validate_bufnr(bufnr or 0)
  local clients = util.get_lsp_clients { bufnr = bufnr, name = 'rustowlsp' }

  for _, client in ipairs(clients) do
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
end

---@param bufnr? number
function M.clear_highlight(bufnr)
  vim.api.nvim_buf_clear_namespace(bufnr or 0, hl_ns, 0, -1)
end

return M
