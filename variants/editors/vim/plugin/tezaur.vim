" TEZAUR for Vim 8+: cited answers from your own library (needs curl and a running `tezaur serve`).
"   :TezaurAsk what is the burden of proof?   :TezaurCompare ...   :TezaurGaps ...   :TezaurSearch terms
"   visual select + :TezaurAskSelection       :TezaurInsert   (puts the last answer under the cursor)
" Settings: g:tezaur_server (default http://127.0.0.1:8787), g:tezaur_token, g:tezaur_k
if exists('g:loaded_tezaur') | finish | endif
let g:loaded_tezaur = 1
command! -nargs=+ TezaurAsk     call tezaur#run('ask', <q-args>)
command! -nargs=+ TezaurCompare call tezaur#run('compare', <q-args>)
command! -nargs=+ TezaurGaps    call tezaur#run('gaps', <q-args>)
command! -nargs=+ TezaurSearch  call tezaur#run('search', <q-args>)
command! -range TezaurAskSelection call tezaur#run('ask', tezaur#selection())
command! TezaurInsert call tezaur#insert()
