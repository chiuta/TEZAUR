
'use strict';
const $=id=>document.getElementById(id);
const esc=s=>String(s).replace(/[&<>"]/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c]));
const ls={get(k,d){try{const v=localStorage.getItem(k);return v==null?d:v}catch(e){return d}},set(k,v){try{localStorage.setItem(k,v)}catch(e){}}};

/* ---------- prompts (same rules as the Rust version) ---------- */
const SYSTEM=`You are a careful academic writing assistant helping a researcher write a thesis. You are given numbered SOURCE EXCERPTS from the researcher's own library. Rules:
1. Base every factual claim ONLY on the excerpts. Never use outside knowledge for facts, figures, dates or quotations.
2. After each claim, cite the excerpt number(s) in square brackets, like [3] or [2, 5]. Never invent a number.
3. If the excerpts do not contain enough to answer, say exactly what is missing instead of guessing.
4. If excerpts disagree, present both positions and cite each.
5. Keep quotations short and exact; otherwise paraphrase in scholarly prose.
6. Write in the same language as the TASK unless told otherwise.`;
const SYSTEM_MERGE=`You merge partial summaries of one document into a single coherent summary. Rules:
1. Use ONLY what the partial summaries say; add no facts, interpretation or outside knowledge.
2. Keep every reference such as (p. 12) or (§4) exactly as written, attached to the claim it supports.
3. Remove repetition, keep the document's own order, and write in the same language as the partials.`;
const SYSTEM_QUERIES=`You turn a research question into search queries for a document library. Output 2 to 4 short, self-contained queries, one per line, no numbering, no commentary. Each query should cover a different aspect of the question. Use the same language as the question.`;

/* ---------- text utils ---------- */
const fold=s=>s.toLowerCase().normalize('NFD').replace(/[\u0300-\u036f]/g,'').replace(/ß/g,'ss').replace(/ø/g,'o').replace(/đ/g,'d').replace(/ł/g,'l');
const STEM=7;
function tokens(s){return (fold(s).match(/[a-z0-9]+/g)||[]).filter(t=>t.length>=2).map(t=>t.slice(0,STEM))}
function fnv(s){let h=0x811c9dc5;for(let i=0;i<s.length;i++){h^=s.charCodeAt(i);h=Math.imul(h,0x01000193)>>>0}return h}
function decodeEnt(s){const t=document.createElement('textarea');t.innerHTML=s;return t.value}
function htmlText(h){return decodeEnt(h.replace(/<(script|style|head)[\s\S]*?<\/\1>/gi,' ').replace(/<\/(p|div|h\d|li|tr|section)>|<br\s*\/?>/gi,'\n').replace(/<[^>]+>/g,' ')).replace(/[ \t\f\r]+/g,' ').replace(/ *\n */g,'\n').replace(/\n{3,}/g,'\n\n')}
function chunkWords(text,size=200,overlap=40){
  const w=text.split(/\s+/).filter(Boolean);if(!w.length)return[];
  const out=[];let s=0;
  while(s<w.length){
    let e=Math.min(s+size,w.length);
    if(e<w.length){for(let j=e;j>Math.max(s+size-40,s+20);j--){if(/[.!?…]["')\]»”]?$/.test(w[j-1])){e=j;break}}}
    out.push(w.slice(s,e).join(' '));
    if(e>=w.length)break;
    s=Math.max(e-overlap,s+1);
  }
  return out;
}

/* ---------- zip (docx / epub) ---------- */
async function unzip(buf){
  const dv=new DataView(buf),u8=new Uint8Array(buf),td=new TextDecoder();
  let e=buf.byteLength-22;while(e>=0&&dv.getUint32(e,true)!==0x06054b50)e--;
  if(e<0)throw new Error('arhivă zip invalidă');
  const n=dv.getUint16(e+10,true);let p=dv.getUint32(e+16,true);const files={};
  for(let i=0;i<n;i++){
    if(dv.getUint32(p,true)!==0x02014b50)break;
    const method=dv.getUint16(p+10,true),csize=dv.getUint32(p+20,true),nl=dv.getUint16(p+28,true),xl=dv.getUint16(p+30,true),cl=dv.getUint16(p+32,true),lho=dv.getUint32(p+42,true);
    files[td.decode(u8.subarray(p+46,p+46+nl))]={method,csize,lho};p+=46+nl+xl+cl;
  }
  return {names:Object.keys(files),async read(name){
    const f=files[name];if(!f)return null;
    const start=f.lho+30+dv.getUint16(f.lho+26,true)+dv.getUint16(f.lho+28,true);
    const data=u8.slice(start,start+f.csize);
    if(f.method===0)return td.decode(data);
    const b=await new Response(new Blob([data]).stream().pipeThrough(new DecompressionStream('deflate-raw'))).arrayBuffer();
    return td.decode(b);
  }};
}
function normPath(p){const o=[];for(const s of p.split('/')){if(s==='..')o.pop();else if(s&&s!=='.')o.push(s)}return o.join('/')}

/* ---------- readers: return [{page, text}] ---------- */
let pdfjsP=null;
function loadPdfjs(){
  if(window.pdfjsLib)return Promise.resolve(window.pdfjsLib);
  if(pdfjsP)return pdfjsP;
  const base='https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/';
  pdfjsP=new Promise((ok,no)=>{const s=document.createElement('script');s.src=base+'pdf.min.js';
    s.onload=()=>{pdfjsLib.GlobalWorkerOptions.workerSrc=base+'pdf.worker.min.js';ok(pdfjsLib)};
    s.onerror=()=>{pdfjsP=null;no(new Error('PDF-urile cer biblioteca pdf.js (se încarcă o singură dată de pe internet). Conectează-te sau convertește PDF-ul în .txt/.docx.'))};
    document.head.appendChild(s)});
  return pdfjsP;
}
async function readPages(file){
  const ext=file.name.split('.').pop().toLowerCase();
  if(['txt','md','markdown','tex'].includes(ext))return[{page:0,text:await file.text()}];
  if(['html','htm'].includes(ext))return[{page:0,text:htmlText(await file.text())}];
  if(ext==='docx'){
    const z=await unzip(await file.arrayBuffer());const x=await z.read('word/document.xml');if(!x)throw new Error('DOCX fără document.xml');
    const paras=x.split('</w:p>').map(p=>decodeEnt([...p.matchAll(/<w:t[^>]*>([\s\S]*?)<\/w:t>/g)].map(m=>m[1]).join('')));
    return[{page:0,text:paras.filter(Boolean).join('\n')}];
  }
  if(ext==='epub'){
    const z=await unzip(await file.arrayBuffer());const P=new DOMParser();
    const cont=await z.read('META-INF/container.xml');let order=[];
    try{
      const opfPath=P.parseFromString(cont,'text/xml').querySelector('rootfile').getAttribute('full-path');
      const opf=P.parseFromString(await z.read(opfPath),'text/xml');const dir=opfPath.includes('/')?opfPath.replace(/\/[^/]*$/,'/'):'';
      const man={};opf.querySelectorAll('manifest > item').forEach(i=>man[i.getAttribute('id')]=i.getAttribute('href'));
      opf.querySelectorAll('spine > itemref').forEach(r=>{const h=man[r.getAttribute('idref')];if(h)order.push(normPath(dir+decodeURIComponent(h.split('#')[0])))});
    }catch(e){order=[]}
    if(!order.length)order=z.names.filter(n=>/\.(x?html?)$/i.test(n)).sort();
    const parts=[];for(const n of order){const h=await z.read(n);if(h)parts.push(htmlText(h))}
    return[{page:0,text:parts.join('\n\n')}];
  }
  if(ext==='pdf'){
    const lib=await loadPdfjs();const pdf=await lib.getDocument({data:new Uint8Array(await file.arrayBuffer())}).promise;const out=[];
    for(let i=1;i<=pdf.numPages;i++){const pg=await pdf.getPage(i);const c=await pg.getTextContent();out.push({page:i,text:c.items.map(t=>t.str+(t.hasEOL?'\n':' ')).join('')})}
    if(!out.some(p=>p.text.trim()))throw new Error('PDF fără strat de text (scanat) — rulează OCR întâi');
    return out;
  }
  throw new Error('format nesuportat: .'+ext);
}

/* ---------- store + BM25 ---------- */
const S={docs:[],chunks:[],bib:[],inv:null,avg:0,only:new Set()};
const DB={
  open(){return new Promise((ok,no)=>{const r=indexedDB.open('tezaur',1);r.onupgradeneeded=()=>r.result.createObjectStore('kv');r.onsuccess=()=>ok(r.result);r.onerror=()=>no(r.error)})},
  async get(k){const d=await this.open();return new Promise((ok,no)=>{const q=d.transaction('kv').objectStore('kv').get(k);q.onsuccess=()=>ok(q.result);q.onerror=()=>no(q.error)})},
  async put(k,v){const d=await this.open();return new Promise((ok,no)=>{const t=d.transaction('kv','readwrite');t.objectStore('kv').put(v,k);t.oncomplete=ok;t.onerror=()=>no(t.error)})}
};
async function persist(){try{await DB.put('index',{docs:S.docs,chunks:S.chunks.map(c=>({doc:c.doc,page:c.page,text:c.text})),bib:S.bibText||[]})}catch(e){console.warn('index not saved',e)}}
function build(){
  const inv=new Map();let total=0;
  S.chunks.forEach((c,i)=>{
    const t=tokens(c.text);c.len=t.length;total+=t.length;const tf=new Map();t.forEach(x=>tf.set(x,(tf.get(x)||0)+1));
    for(const [x,n] of tf){let a=inv.get(x);if(!a)inv.set(x,a=[]);a.push([i,n])}
  });
  S.inv=inv;S.avg=total/Math.max(1,S.chunks.length);
}
function allowed(){return S.only}
function search(query,k=8,perDoc=3){
  const qt=[...new Set(tokens(query))];if(!qt.length||!S.inv)return[];
  const N=S.chunks.length,sc=new Map(),ok=allowed();
  for(const t of qt){
    const p=S.inv.get(t);if(!p)continue;const idf=Math.log(1+(N-p.length+.5)/(p.length+.5));
    for(const [i,tf] of p){const c=S.chunks[i];if(!ok.has(c.doc))continue;
      sc.set(i,(sc.get(i)||0)+idf*tf*2.2/(tf+1.2*(.25+.75*c.len/S.avg)))}
  }
  const ranked=[...sc].sort((a,b)=>b[1]-a[1]);const cnt={},out=[];
  for(const [i,s] of ranked){const d=S.chunks[i].doc;if((cnt[d]||0)>=perDoc)continue;cnt[d]=(cnt[d]||0)+1;out.push({chunk:i,score:s});if(out.length>=k)break}
  return out;
}
function coverage(query,text){const q=[...new Set(tokens(query))];if(!q.length)return 1;const t=new Set(tokens(text));return q.filter(x=>t.has(x)).length/q.length}

/* ---------- BibTeX ---------- */
function bibClean(s){return s.replace(/\\[`'"^~=.]/g,'').replace(/\\([&%_#$])/g,'$1').replace(/\\[a-zA-Z]+/g,'').replace(/[{}]/g,'').replace(/\s+/g,' ').trim()}
function parseBib(text){
  const out=[];let i=0;
  while((i=text.indexOf('@',i))>=0){
    const m=/^@(\w+)\s*\{/.exec(text.slice(i,i+40));if(!m){i++;continue}
    let j=i+m[0].length,d=1;const st=j;
    while(j<text.length&&d>0){if(text[j]==='{')d++;else if(text[j]==='}')d--;j++}
    i=j;if(/^(comment|string|preamble)$/i.test(m[1]))continue;
    const body=text.slice(st,j-1),c=body.indexOf(',');if(c<0)continue;
    const e={key:body.slice(0,c).trim(),author:'',year:'',title:'',container:'',doi:'',file:''};
    const rest=body.slice(c+1);const re=/([A-Za-z\-_]+)\s*=\s*/g;let f;
    while((f=re.exec(rest))){
      let p=re.lastIndex,v='';
      if(rest[p]==='{'){let dd=0,q=p;for(;q<rest.length;q++){if(rest[q]==='{')dd++;else if(rest[q]==='}'){dd--;if(!dd)break}}v=rest.slice(p+1,q);re.lastIndex=q+1}
      else if(rest[p]==='"'){let q=p+1,dd=0;for(;q<rest.length&&!(rest[q]==='"'&&!dd);q++){if(rest[q]==='{')dd++;else if(rest[q]==='}')dd--}v=rest.slice(p+1,q);re.lastIndex=q+1}
      else{let q=rest.indexOf(',',p);if(q<0)q=rest.length;v=rest.slice(p,q);re.lastIndex=q}
      const n=f[1].toLowerCase(),cv=bibClean(v);
      if(n==='author')e.author=cv;else if(n==='editor'&&!e.author)e.author=cv;else if(n==='year')e.year=cv;
      else if(n==='date'&&!e.year)e.year=cv.slice(0,4);else if(n==='title')e.title=cv;
      else if(['journal','journaltitle','booktitle'].includes(n))e.container=cv;
      else if(['publisher','school','institution'].includes(n)&&!e.container)e.container=cv;
      else if(n==='doi')e.doi=cv;else if(n==='file')e.file=v.trim();
    }
    out.push(e);
  }
  return out;
}
const ORG=['of','for','the','council','university','institute','commission','union','association','ministry','organization','organisation'];
function lastName(a){a=a.trim();if(a.includes(','))return a.split(',')[0].trim();
  if(a.toLowerCase().split(/\s+/).some(w=>ORG.includes(w)))return a;return a.split(/\s+/).pop()}
function bibCite(e){
  const a=e.author.split(' and ').map(x=>x.trim()).filter(Boolean);
  const who=a.length===0?'':a.length===1?lastName(a[0]):a.length===2?lastName(a[0])+' & '+lastName(a[1]):lastName(a[0])+' et al.';
  if(!who)return e.title||e.key;return e.year?`${who} (${e.year})`:who;
}
function bibRef(e){let s='';if(e.author)s+=e.author.split(' and ').join('; ')+'. ';if(e.year)s+=`(${e.year}). `;if(e.title)s+=e.title+'. ';if(e.container)s+=e.container+'. ';if(e.doi)s+='https://doi.org/'+e.doi;return s.trim()}
function bibFind(path){
  const fname=path.split('/').pop().toLowerCase(),stem=fname.replace(/\.[^.]+$/,'');
  return S.bib.find(e=>e.key.toLowerCase()===stem)||S.bib.find(e=>e.file&&e.file.toLowerCase().includes(fname))||null;
}
function label(h){const c=S.chunks[h.chunk],d=S.docs[c.doc],e=bibFind(d.path);const n=e?bibCite(e):d.title;return c.page>0?`${n}, p. ${c.page}`:n}

/* ---------- citations ---------- */
const GROUP=/\[(\d+(?:\s*[,;]\s*\d+)*)\]/g;
function citedNumbers(t){const s=new Set();for(const m of t.matchAll(GROUP))m[1].split(/[,;]/).forEach(x=>s.add(+x.trim()));return s}
function rewriteCites(t,labels){return t.replace(GROUP,(a,g)=>'('+g.split(/[,;]/).map(x=>{x=+x.trim();return x>=1&&x<=labels.length?labels[x-1]:'?'}).join('; ')+')')}
function checkAnswer(hits,ans){
  const w=[],cited=citedNumbers(ans);
  const bad=[...cited].filter(n=>n<1||n>hits.length);
  if(bad.length)w.push(`modelul a citat pasaje inexistente: ${bad.join(', ')} — consideră afirmațiile din apropiere nesusținute`);
  if(!cited.size)w.push('nicio citare — nu te baza pe acest text');
  const un=ans.split(/\n\n/).filter(p=>p.length>200&&!citedNumbers(p).size).length;
  if(un)w.push(`${un} paragraf(e) lungi fără citare — verifică sau elimină`);
  return w;
}
function userPrompt(hits,task,lang){
  let s='SOURCE EXCERPTS:\n\n';hits.forEach((h,i)=>{s+=`[${i+1}] (${label(h)})\n${S.chunks[h.chunk].text}\n\n`});
  s+='TASK:\n'+task;if(lang)s+=`\n\nWrite the answer in: ${lang}.`;return s;
}

/* ---------- LLM ---------- */
const DEF={ollama:'http://localhost:11434',jan:'http://127.0.0.1:1337/v1',openai:'http://127.0.0.1:8080/v1'};
let ctrl=null;
const cfg=()=>({api:$('api').value,host:$('host').value.trim().replace(/\/+$/,''),model:$('model').value.trim()||$('modelSel').value,key:$('key').value.trim(),numctx:+$('numctx').value||8192});
function hint(c){return `Nu pot ajunge la ${c.host}. Verifică: (1) serverul rulează; (2) permite accesul din browser (CORS): `+
  (c.api==='ollama'?'pornește Ollama cu variabila OLLAMA_ORIGINS="*"':c.api==='jan'?'în Jan: Settings → Local API Server → activează CORS':'activează CORS în server (LM Studio: Enable CORS; llama-server: --api-key nu e necesar, dar cere antet CORS)')+
  '. Pagina deschisă din fișier (file://) are originea „null”, deci CORS trebuie să accepte orice origine.'}
async function* lines(resp){const r=resp.body.getReader(),d=new TextDecoder();let b='';for(;;){const {done,value}=await r.read();if(done)break;b+=d.decode(value,{stream:true});let i;while((i=b.indexOf('\n'))>=0){yield b.slice(0,i);b=b.slice(i+1)}}if(b.trim())yield b}
async function http(path,opt={}){
  const c=cfg();const headers={'Content-Type':'application/json'};if(c.key)headers.Authorization='Bearer '+c.key;
  let r;try{r=await fetch(c.host+path,{...opt,headers})}catch(e){if(e.name==='AbortError')throw e;throw new Error(hint(c))}
  if(!r.ok){const t=await r.text().catch(()=>'');throw new Error(`Serverul a răspuns ${r.status}: ${t.slice(0,300)}`)}
  return r;
}
async function chat(sys,user,onTok){
  const c=cfg();if(!c.model)throw new Error('Alege un model (butonul „Listează”).');
  const messages=[{role:'system',content:sys},{role:'user',content:user}];let full='';
  ctrl=new AbortController();
  if(c.api==='ollama'){
    const r=await http('/api/chat',{method:'POST',signal:ctrl.signal,body:JSON.stringify({model:c.model,messages,stream:true,options:{num_ctx:c.numctx}})});
    for await(const l of lines(r)){if(!l.trim())continue;const j=JSON.parse(l);if(j.error)throw new Error(j.error);const t=j.message&&j.message.content;if(t){full+=t;onTok&&onTok(full)}}
  }else{
    const r=await http('/chat/completions',{method:'POST',signal:ctrl.signal,body:JSON.stringify({model:c.model,messages,stream:true})});
    for await(const l of lines(r)){if(!l.startsWith('data:'))continue;const d=l.slice(5).trim();if(d==='[DONE]')break;const j=JSON.parse(d);const t=j.choices&&j.choices[0]&&j.choices[0].delta&&j.choices[0].delta.content;if(t){full+=t;onTok&&onTok(full)}}
  }
  return full;
}
async function listModels(){
  const c=cfg();const r=await http(c.api==='ollama'?'/api/tags':'/models');const j=await r.json();
  return c.api==='ollama'?(j.models||[]).map(m=>m.name):(j.data||[]).map(m=>m.id);
}

/* ---------- UI: library ---------- */
function renderLib(){
  const ul=$('docs');ul.innerHTML='';
  S.docs.forEach((d,i)=>{const li=document.createElement('li');
    li.innerHTML=`<input type="checkbox" ${S.only.has(i)?'checked':''}><span title="${esc(d.path)}">${esc(d.title)}</span><button class="x" title="Șterge">×</button>`;
    li.firstChild.onchange=e=>{e.target.checked?S.only.add(i):S.only.delete(i)};
    li.lastChild.onclick=()=>removeDoc(i);ul.appendChild(li)});
  $('status').textContent=S.docs.length?`${S.docs.length} documente · ${S.chunks.length} pasaje`:'Biblioteca e goală.';
  $('docSel').innerHTML=S.docs.map((d,i)=>`<option value="${i}">${esc(d.title)}</option>`).join('');
}
async function removeDoc(i){
  S.docs.splice(i,1);S.chunks=S.chunks.filter(c=>c.doc!==i);S.chunks.forEach(c=>{if(c.doc>i)c.doc--});
  S.only=new Set(S.docs.map((_,j)=>j));build();renderLib();await persist();
}
async function ingest(files){
  const ok=/\.(pdf|docx|epub|txt|md|markdown|html?|tex)$/i;const list=[...files].filter(f=>ok.test(f.name));
  const msgs=[];let n=0;
  for(const f of list){
    const path=f.webkitRelativePath||f.name;$('status').textContent=`Se citește (${++n}/${list.length}): ${f.name}…`;
    try{
      const pages=await readPages(f);const hash=fnv(pages.map(p=>p.text).join('\u0001'));
      const old=S.docs.findIndex(d=>d.path===path);
      if(old>=0){if(S.docs[old].hash===hash)continue;await removeDoc(old)}
      const di=S.docs.length;let cnt=0;
      for(const p of pages)for(const t of chunkWords(p.text)){S.chunks.push({doc:di,page:p.page,text:t});cnt++}
      if(!cnt){msgs.push(`${f.name}: fără text`);continue}
      S.docs.push({path,title:f.name.replace(/\.[^.]+$/,''),hash,pages:pages.length});S.only.add(di);
    }catch(e){msgs.push(`${f.name}: ${e.message}`)}
  }
  build();renderLib();await persist();
  if(msgs.length)$('status').innerHTML+='<br><span class="err">'+msgs.map(esc).join('<br>')+'</span>';
}
$('addFiles').onclick=()=>$('fFiles').click();$("fDir").webkitdirectory=true;$('addDir').onclick=()=>$('fDir').click();
$('fFiles').onchange=e=>{ingest(e.target.files);e.target.value=''};$('fDir').onchange=e=>{ingest(e.target.files);e.target.value=''};
$('selAll').onclick=()=>{S.only=new Set(S.docs.map((_,i)=>i));renderLib()};$('selNone').onclick=()=>{S.only=new Set();renderLib()};
$('clearAll').onclick=async()=>{if(!confirm('Ștergi toată biblioteca din acest browser?'))return;S.docs=[];S.chunks=[];S.only=new Set();build();renderLib();await persist()};
$('addBib').onclick=()=>$('fBib').click();
$('fBib').onchange=async e=>{const texts=[];for(const f of e.target.files)texts.push(await f.text());S.bibText=(S.bibText||[]).concat(texts);S.bib=S.bibText.flatMap(parseBib);bibStat();await persist();e.target.value=''};
function bibStat(){const m=S.docs.filter(d=>bibFind(d.path)).length;$('bibStatus').textContent=S.bib.length?`${S.bib.length} intrări BibTeX · ${m}/${S.docs.length} documente asociate (cheie = nume fișier, sau câmpul file)`:'Fără BibTeX: citările folosesc titlul fișierului.'}

/* ---------- UI: settings ---------- */
function loadCfg(){
  $('api').value=ls.get('t.api','ollama');$('host').value=ls.get('t.host',DEF[$('api').value]);$('model').value=ls.get('t.model','');
  $('numctx').value=ls.get('t.numctx','8192');$('key').value=ls.get('t.key','');
}
['api','host','model','numctx','key'].forEach(id=>$(id).addEventListener('change',()=>ls.set('t.'+id,$(id).value)));
$('api').addEventListener('change',()=>{$('host').value=DEF[$('api').value];ls.set('t.host',$('host').value);$('modelSel').innerHTML=''});
$('listModels').onclick=async()=>{try{const m=await listModels();$('modelSel').innerHTML=m.map(x=>`<option>${esc(x)}</option>`).join('');if(m.length&&!$('model').value){$('model').value=m[0];ls.set('t.model',m[0])}$('doctorOut').innerHTML=`<span class="ok">${m.length} modele găsite.</span>`}catch(e){$('doctorOut').innerHTML='<span class="err">'+esc(e.message)+'</span>'}};
$('modelSel').onchange=()=>{$('model').value=$('modelSel').value;ls.set('t.model',$('model').value)};
$('doctor').onclick=async()=>{
  const o=[];const c=cfg();
  try{const m=await listModels();o.push(`<span class="ok">✓ server la ${esc(c.host)} (${m.length} modele)</span>`);
    if(!c.model)o.push('<span class="warn">! niciun model ales</span>');else if(!m.some(x=>x===c.model||x.startsWith(c.model+':')))o.push(`<span class="warn">! modelul „${esc(c.model)}” nu e în listă</span>`)}
  catch(e){o.push('<span class="err">✗ '+esc(e.message)+'</span>')}
  o.push(S.chunks.length?`<span class="ok">✓ index: ${S.docs.length} documente, ${S.chunks.length} pasaje</span>`:'<span class="warn">! biblioteca e goală — adaugă fișiere</span>');
  if(S.chunks.length&&!S.only.size)o.push('<span class="warn">! niciun document bifat</span>');
  $('doctorOut').innerHTML=o.join('<br>');
};

/* ---------- modes ---------- */
const MODES={
  search:{n:'Căutare',h:'Doar pasajele relevante, fără model. Arată % din termenii interogării găsiți.',ph:'Ce cauți?'},
  ask:{n:'Întreabă',h:'Răspuns cu citări [n] doar din pasajele regăsite.',ph:'Întrebarea ta…',dec:1},
  compare:{n:'Compară',h:'Poziții, acorduri și dezacorduri între surse.',ph:'Cum este definit X?'},
  gaps:{n:'Lacune',h:'Ce e stabilit, ce e slab susținut, ce rămâne fără răspuns.',ph:'Tema…'},
  summarize:{n:'Rezumat',h:'Rezumatul unui întreg document (map-reduce, lent), cu referințe de pagină.',doc:1},
  outline:{n:'Schiță',h:'Schelet de capitol; marchează secțiunile fără surse.',ph:'Titlul capitolului…'},
  draft:{n:'Redactare',h:'Redactare din surse, cu citări. Tipul schimbă cerința, nu regulile.',ph:'Titlul secțiunii…',kind:1,notes:1,dec:1},
  flashcards:{n:'Carduri',h:'Carduri de studiu; se păstrează doar cele al căror răspuns citează un pasaj real. Export Anki (.tsv).',ph:'Subiectul…'}
};
let mode='ask',lastText='',lastTsv='';
function setMode(m){
  mode=m;const M=MODES[m];
  [...$('tabs').children].forEach(b=>b.classList.toggle('on',b.dataset.m===m));
  $('modeHelp').textContent=M.h;$('fInput').hidden=!!M.doc;$('q').placeholder=M.ph||'';
  $('fNotes').hidden=!M.notes;$('fKind').hidden=!M.kind;$('fDoc').hidden=!M.doc;$('fDec').hidden=!M.dec;
  $('run').textContent=m==='search'?'Caută':'Rulează';
}
Object.keys(MODES).forEach(m=>{const b=document.createElement('button');b.textContent=MODES[m].n;b.dataset.m=m;b.onclick=()=>setMode(m);$('tabs').appendChild(b)});

function mdRender(t,n){
  const inl=s=>esc(s).replace(/\*\*(.+?)\*\*/g,'<b>$1</b>').replace(/(^|[\s(])\*([^*\s][^*]*?)\*(?=[\s).,;:]|$)/g,'$1<i>$2</i>')
    .replace(GROUP,(a,g)=>'['+g.split(/[,;]/).map(x=>{x=+x.trim();return `<a class="c${x<1||x>n?' bad':''}" data-n="${x}">${x}</a>`}).join(',')+']');
  let h='',list=false;
  for(const raw of t.split('\n')){
    const l=raw.trimEnd();let m;
    if((m=/^\s*(?:[-*•]|\d+[.)])\s+(.*)/.exec(l))){if(!list){h+='<ul>';list=true}h+='<li>'+inl(m[1])+'</li>';continue}
    if(list){h+='</ul>';list=false}
    if((m=/^#{1,4}\s+(.*)/.exec(l)))h+='<h3>'+inl(m[1])+'</h3>';else if(l.trim())h+='<p>'+inl(l)+'</p>';
  }
  return h+(list?'</ul>':'');
}
function showHits(hits,withScore){
  $('srcCard').hidden=!hits.length;
  $('src').innerHTML=hits.map((h,i)=>`<div class="ex" id="ex${i+1}"><b>[${i+1}] ${esc(label(h))}</b>${withScore?` <span class="small">· ${(h.cov*100|0)}% termeni</span>`:''}<br>${esc(S.chunks[h.chunk].text)}</div>`).join('');
}
$('out').addEventListener('click',e=>{const a=e.target.closest('a.c');if(!a)return;const el=$('ex'+a.dataset.n);if(!el)return;
  document.querySelectorAll('.ex.hl').forEach(x=>x.classList.remove('hl'));el.classList.add('hl');el.scrollIntoView({behavior:'smooth',block:'center'})});
function refsFor(hits,ans){
  const out=[];for(const n of [...citedNumbers(ans)].sort((a,b)=>a-b)){if(n<1||n>hits.length)continue;const h=hits[n-1],d=S.docs[S.chunks[h.chunk].doc],e=bibFind(d.path);
    out.push(`[${n}] ${label(h)} — ${e?bibRef(e)+'  ['+d.path+']':d.path}`)}
  return out;
}
function warnBox(ws){$('warns').innerHTML=ws.map(w=>`<div class="warn">⚠ ${esc(w)}</div>`).join('')}
function show(text,nHits){lastText=text;$('resCard').hidden=false;$('out').innerHTML=mdRender(text,nHits)}

function retrieve(queries,k){
  let lists=queries.map(q=>search(q,k*2,3));
  if(lists.length===1)lists=[lists[0].slice(0,k)];
  const seen=new Set(),hits=[];
  for(let r=0;hits.length<k;r++){let any=false;for(const l of lists){if(r<l.length){any=true;if(!seen.has(l[r].chunk)){seen.add(l[r].chunk);hits.push(l[r])}if(hits.length>=k)break}}if(!any)break}
  const q=queries.join(' ');hits.forEach(h=>h.cov=coverage(q,S.chunks[h.chunk].text));
  return hits;
}
async function queriesFor(q,dec){
  if(!dec)return[q];
  $('runInfo').textContent='descompun întrebarea…';
  const t=await chat(SYSTEM_QUERIES,q);const l=t.split('\n').map(x=>x.replace(/^[\s\-*\d.)]+/,'').trim()).filter(Boolean).slice(0,4);
  return l.length?[q,...l]:[q];
}
const TASKS={
  ask:q=>q,
  compare:q=>`Compare how the different sources address the following. Structure: (1) the positions found, with which sources hold them; (2) points of agreement; (3) points of disagreement. Cite every statement.\n\n${q}`,
  gaps:q=>`For the topic below, list: what the excerpts establish well; what is only weakly supported; what the excerpts leave unanswered; what kinds of sources seem to be missing. Cite where applicable.\n\n${q}`,
  outline:q=>`Propose a chapter outline (numbered sections and subsections, one line each) for: ${q}. Base each section on the excerpts and cite them. Mark with [NO SOURCES] any section the excerpts do not support.`,
  flashcards:q=>`Create 8 to 12 study flashcards on: ${q}. Output ONLY lines of the form:\nQ: question | A: short answer [n]\nEvery answer must end with the number(s) of the excerpt(s) it comes from. Use only the excerpts.`
};
function draftTask(kind,title,notes){
  const base={section:`Write a thesis section titled "${title}".`,
    introduction:`Write the introduction of a thesis chapter titled "${title}": context, the problem the sources address, and the structure to follow. Where the author's own aims or research questions would go, leave a placeholder [AUTHOR: …] instead of inventing them.`,
    'lit-review':`Write a literature review on "${title}", organised by themes rather than source by source, showing how the sources relate.`,
    'critical-review':`Write a critical review on "${title}": strengths, limits and contested points of the sources.`}[kind];
  return base+' Scholarly prose, every factual claim cited.'+(notes.trim()?`\n\nThe author's notes (guidance, not facts):\n${notes}`:'');
}
function parseCards(t){
  const out=[];for(const raw of t.split('\n')){
    const l=raw.trim().replace(/^[\d\s\-*•.)]+/,'');const m=/q:\s*(.*?)\s*\|\s*a:\s*(.+)$/i.exec(l);
    if(m&&m[1]&&m[2])out.push([m[1],m[2]]);
  }return out;
}
function setBusy(b){$('run').disabled=b;$('stop').disabled=!b;if(!b)$('runInfo').textContent=''}
$('stop').onclick=()=>{if(ctrl)ctrl.abort()};

async function summarize(di){
  const idx=S.chunks.map((c,i)=>c.doc===di?i:-1).filter(i=>i>=0);const B=6,parts=[];
  for(let b=0;b<idx.length;b+=B){
    $('runInfo').textContent=`rezum partea ${b/B+1}/${Math.ceil(idx.length/B)}…`;
    const hits=idx.slice(b,b+B).map(i=>({chunk:i,score:0}));
    const labels=hits.map(h=>{const c=S.chunks[h.chunk];return c.page>0?`p. ${c.page}`:`§${idx.indexOf(h.chunk)+1}`});
    const t=await chat(SYSTEM,userPrompt(hits,'Summarize these excerpts faithfully in at most 150 words. Cite the excerpts you rely on.'),null);
    parts.push(rewriteCites(t,labels));
  }
  let level=parts;
  while(level.length>1){
    const next=[];for(let i=0;i<level.length;i+=6){
      const g=level.slice(i,i+6);$('runInfo').textContent=`combin rezumatele (${level.length} → ${Math.ceil(level.length/6)})…`;
      next.push(g.length===1?g[0]:await chat(SYSTEM_MERGE,'PARTIAL SUMMARIES:\n\n'+g.map((p,j)=>`${j+1}. ${p}`).join('\n\n')+'\n\nMerge them into one summary.',null));
    }level=next;
  }
  return level[0]||'';
}

async function run(){
  if(!S.chunks.length)return alert('Adaugă mai întâi documente în bibliotecă.');
  if(!S.only.size)return alert('Niciun document nu e bifat.');
  const q=$('q').value.trim(),k=Math.max(1,+$('k').value||8),lang=$('lang').value.trim();
  $('dlTsv').hidden=true;$('refs').textContent='';warnBox([]);
  try{
    setBusy(true);
    if(mode==='summarize'){
      const di=+$('docSel').value;$('srcCard').hidden=true;
      const t=await summarize(di);show(t,0);
      const pages=new Set(S.chunks.filter(c=>c.doc===di).map(c=>c.page));
      const bad=[...t.matchAll(/p\. (\d+)/g)].map(m=>+m[1]).filter(p=>!pages.has(p));
      warnBox(bad.length?[`pagini citate care nu există în document: ${[...new Set(bad)].join(', ')}`]:[]);
      return;
    }
    if(!q)return alert('Scrie întrebarea sau tema.');
    if(mode==='search'){
      const hits=retrieve([q],k);show(hits.length?`${hits.length} pasaje găsite.`:'Niciun pasaj potrivit.',0);showHits(hits,true);return;
    }
    const queries=await queriesFor(q,$('dec').checked&&!$('fDec').hidden);
    const hits=retrieve(queries,k);
    if(!hits.length){show('Niciun pasaj relevant în documentele bifate.',0);$('srcCard').hidden=true;return}
    showHits(hits,true);
    const ws=[];if(Math.max(...hits.map(h=>h.cov))<.4)ws.push('potrivire slabă: cel mai bun pasaj conține sub 40% din termenii interogării — răspunsul poate fi superficial');
    let task;
    if(mode==='draft')task=draftTask($('kind').value,q,$('notes').value);else task=TASKS[mode](q);
    $('runInfo').textContent='generez…';let pend=false,latest='';
    const ans=await chat(SYSTEM,userPrompt(hits,task,lang),t=>{latest=t;if(!pend){pend=true;requestAnimationFrame(()=>{pend=false;if(mode!=='flashcards')show(latest,hits.length)})}});
    latest=ans;
    if(mode==='flashcards'){
      const cards=parseCards(ans),labels=hits.map(label);let drop=0;const keep=[];
      for(const [f,b] of cards){const nums=[...citedNumbers(b)];if(nums.length&&nums.every(n=>n>=1&&n<=hits.length))keep.push([f,rewriteCites(b,labels)]);else drop++}
      lastTsv='#separator:tab\n#html:false\n'+keep.map(([f,b])=>[f,b].map(x=>x.replace(/[\t\r\n]+/g,' ')).join('\t')).join('\n')+'\n';
      $('resCard').hidden=false;$('out').innerHTML=keep.length?'<table><tr><th>Față</th><th>Verso (cu sursă)</th></tr>'+keep.map(([f,b])=>`<tr><td>${esc(f)}</td><td>${esc(b)}</td></tr>`).join('')+'</table>':'<p>Niciun card valid.</p>';
      lastText=keep.map(([f,b])=>`Q: ${f}\nA: ${b}`).join('\n\n');
      if(drop)ws.push(`${drop} card(uri) eliminate: răspuns fără citare sau cu citare inexistentă`);
      $('dlTsv').hidden=!keep.length;warnBox(ws);return;
    }
    show(ans,hits.length);warnBox(ws.concat(checkAnswer(hits,ans)));
    $('refs').innerHTML='<b>Referințe</b><br>'+refsFor(hits,ans).map(esc).join('<br>');
  }catch(e){
    if(e.name==='AbortError'){warnBox(['oprit de utilizator'])}
    else{$('resCard').hidden=false;$('out').innerHTML='<div class="err">'+esc(e.message)+'</div>'}
  }finally{setBusy(false)}
}
$('run').onclick=run;
$('q').addEventListener('keydown',e=>{if(e.key==='Enter'&&(e.ctrlKey||e.metaKey))run()});
function dl(name,text,type){
  if(window.AndroidBridge){const r=AndroidBridge.save(name,type,text);alert(r==='ok'?'Salvat în Descărcări: '+name:'Eroare: '+r);return}
  const a=document.createElement('a');a.href=URL.createObjectURL(new Blob([text],{type}));a.download=name;a.click();setTimeout(()=>URL.revokeObjectURL(a.href),2000)}
$('copy').onclick=()=>{navigator.clipboard&&navigator.clipboard.writeText(lastText).catch(()=>{})};
$('dlMd').onclick=()=>dl('tezaur-'+mode+'.md',lastText+($('refs').textContent?'\n\n'+$('refs').innerText:''),'text/markdown');
$('dlTsv').onclick=()=>dl('carduri.tsv',lastTsv,'text/tab-separated-values');

/* ---------- boot ---------- */
(async()=>{
  loadCfg();setMode('ask');
  try{const d=await DB.get('index');if(d){S.docs=d.docs||[];S.chunks=d.chunks||[];S.bibText=d.bib||[];S.bib=S.bibText.flatMap(parseBib);S.only=new Set(S.docs.map((_,i)=>i));build()}}catch(e){}
  renderLib();bibStat();
})();
if('serviceWorker' in navigator&&location.protocol.startsWith('http'))navigator.serviceWorker.register('sw.js').catch(()=>{});
