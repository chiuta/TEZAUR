let s:last = []

function! tezaur#selection() abort
  let [l1, c1] = getpos("'<")[1:2]
  let [l2, c2] = getpos("'>")[1:2]
  let lines = getline(l1, l2)
  if empty(lines) | return '' | endif
  let lines[-1] = lines[-1][: c2 - 1]
  let lines[0] = lines[0][c1 - 1 :]
  return join(lines, ' ')
endfunction

function! s:post(path, body) abort
  let server = get(g:, 'tezaur_server', 'http://127.0.0.1:8787')
  let cmd = ['curl', '-s', '--max-time', '600', '-H', 'Content-Type: application/json']
  if get(g:, 'tezaur_token', '') !=# ''
    call extend(cmd, ['-H', 'Authorization: Bearer ' . g:tezaur_token])
  endif
  call extend(cmd, ['-d', '@-', substitute(server, '/$', '', '') . a:path])
  let out = system(join(map(cmd, 'shellescape(v:val)'), ' '), json_encode(a:body))
  if v:shell_error != 0 || out ==# ''
    throw 'TEZAUR: cannot reach ' . server
  endif
  return json_decode(out)
endfunction

" Pure: turn a server answer into display lines.
function! tezaur#format(mode, d) abort
  if has_key(a:d, 'error') | return ['Error: ' . a:d.error] | endif
  let L = []
  if a:mode ==# 'search'
    for h in a:d.hits
      call add(L, printf('[%d] %s', h.n, h.label))
      call add(L, '    ' . substitute(strpart(h.text, 0, 300), '\n\+', ' ', 'g'))
    endfor
    if empty(L) | call add(L, 'No matching passages.') | endif
    return L
  endif
  call extend(L, split(a:d.answer, "\n", 1))
  if get(a:d, 'references', '') !=# ''
    call extend(L, ['', '## References'] + split(a:d.references, "\n"))
  endif
  for w in get(a:d, 'warnings', [])
    call add(L, '⚠ ' . w)
  endfor
  return L
endfunction

function! tezaur#run(mode, q) abort
  let q = substitute(a:q, '^\s\+\|\s\+$', '', 'g')
  if q ==# '' | echo 'TEZAUR: empty question' | return | endif
  echo 'TEZAUR: thinking… (a local model can take a minute)'
  redraw
  try
    let path = a:mode ==# 'search' ? '/api/search' : '/api/ask'
    let d = s:post(path, {'q': q, 'mode': a:mode, 'k': get(g:, 'tezaur_k', 6)})
  catch
    echohl ErrorMsg | echo v:exception | echohl None
    return
  endtry
  let s:last = tezaur#format(a:mode, d)
  silent! pedit +setlocal\ buftype=nofile\ bufhidden=wipe\ noswapfile\ filetype=markdown TEZAUR
  wincmd P
  silent %delete _
  call setline(1, s:last)
  setlocal nomodifiable
  wincmd p
  echo ''
endfunction

function! tezaur#insert() abort
  if empty(s:last) | echo 'TEZAUR: nothing to insert yet' | return | endif
  call append(line('.'), s:last)
endfunction

function! tezaur#last() abort
  return s:last
endfunction
