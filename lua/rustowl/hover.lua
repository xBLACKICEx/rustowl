local M = {}

function M.create_lsp_attach_autocmd(idle_time_ms)
  local augroup = vim.api.nvim_create_augroup('RustOwlLspAttach', {})
  vim.api.nvim_create_autocmd('LspAttach', {
    group = augroup,
    callback = function(event)
      M.show_on_cursor_hold(event.buf, idle_time_ms)
    end,
  })
end

function M.show_on_cursor_hold(bufnr, idle_time_ms)
  local timer = nil
  local augroup = vim.api.nvim_create_augroup('RustOwlCmd', { clear = true })

  local function clear_timer()
    if timer then
      timer:stop()
      timer:close()
      timer = nil
    end
  end

  local function start_timer()
    clear_timer()
    local t, err = vim.uv.new_timer()
    timer = t
    assert(timer, err)

    timer:start(idle_time_ms, 0, vim.schedule_wrap(function()
      require('rustowl').show(bufnr)
    end))
  end

  vim.api.nvim_create_autocmd({ 'CursorMoved', 'CursorMovedI' }, {
    group = augroup,
    buffer = bufnr,
    callback = function()
      require('rustowl').hide(bufnr)
      start_timer()
    end,
  })

  vim.api.nvim_create_autocmd('BufUnload', {
    group = augroup,
    buffer = bufnr,
    callback = clear_timer,
  })

  start_timer()
end

return M
