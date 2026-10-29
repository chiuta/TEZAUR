# TEZAUR — originalitate și comparație cu concurența (28 octombrie 2026)

> **Actualizare 0.3.0 (aceeași zi):** vezi secțiunea „După 0.3.0” la sfârșit. Tabelul de lipsuri de mai jos descrie starea 0.2.1.

**Metodă și limite.** Am căutat pe web în aceeași zi și am citit paginile accesibile (surse la sfârșit). Multe sunt bloguri de comparație, unele publicate de vânzători
(le-am marcat), iar pe unele pagini nu am putut citi detalii (NotebookLM, `zotero-grounded-qa` — acces blocat în sesiune). „Nu am găsit" nu înseamnă „nu există":
o căutare de o după-amiază nu dovedește absența. Nu am rulat niciun concurent; comparațiile de calitate nu sunt măsurate de mine.

## 1. Peisajul azi

| Categorie | Reprezentanți (din sursele citite) | Ce oferă |
|---|---|---|
| Cloud, cercetare academică | **Elicit** (Pro 49 $/lună, tabele de extracție, 125–138 M lucrări), **Consensus** (400 M lucrări, „Consensus Meter”), **SciSpace**, **Atlas**, **Paperguide**, **Semantic Scholar**, **Research Rabbit** | căutare în literatura mondială, extracție, rezumate; răspunsuri cu surse |
| Cloud, „contextul citărilor” | **Scite** (peste 1,2 mld. de enunțuri de citare clasificate suportă/menționează/contrazice; 12–50 $/lună după sursă; reference-check pentru surse retrase) | cum au fost citate lucrările ulterior |
| Cloud, verificare de citări în teze | **Acurio** (găzduit în UE; verifică existența/retragerea prin Crossref/OpenAlex/DOAJ și, pe PDF-urile încărcate, marchează afirmațiile *supported/partial/unsupported* cu citatul exact; verificarea completă e cu plată) | cel mai apropiat concurent funcțional al `audit` |
| Cercetare academică (articole) | **CiteCheck** (arXiv/alphaXiv, mai 2026: existența și metadatele referințelor, 88,7 macro-F1 pe 982 citări de fizică; nu e clar dacă e publicat ca instrument), alte lucrări de „citation hallucination” | detectarea referințelor inventate, nu a sprijinului pe pagină |
| RAG local, aplicații generale | **AnythingLLM** (MIT; citări cu fișier+pagină, desktop), **PrivateGPT** (Apache 2.0, offline), **Open WebUI** (BSD-3 după sursa citită; verificați licența actuală), **Khoj** (AGPL; versiunea cloud închisă la 15 aprilie 2026, rămâne self-host) | chat cu documente, local; calitate bună, nu specializate pe teze |
| Ecosistemul Zotero | **llm-for-zotero** (AGPL-3, ~3.400 stele, mod agent, MinerU pentru tabele/ecuații), **Zotero RAG Assistant** (GPL-3, 158 stele, panou „Verify” cu trecerea evidențiată), **zotero-mcp** (MIT, v0.3.1 din 7 aug. 2026), Zotero GPT, `zotero-grounded-qa` (menționat pe forum, nevăzut de mine) | chat/căutare semantică în biblioteca Zotero. Pe forum (iunie 2026) încă se cere o funcție nativă de chat — **Zotero nu are AI nativ** după acea discuție |
| NotebookLM | — (cunoștințele mele despre el sunt dinainte de aprilie 2026; nu am putut citi o pagină actuală) | cloud, surse încărcate, citări în text, rezumate audio |

O comparație independentă (PromptQuorum, august 2026, un singur corp de 5.047 de pagini și 50 de întrebări — **deci orientativă**) dă rate de halucinație 6 % (AnythingLLM), 11 % (PrivateGPT), 14 % (Open WebUI)
și arată că toate trei se degradează între 8.000 și 12.000 de pagini pe hardware obișnuit. Nu există măsurătoare comparabilă pentru TEZAUR.

## 2. Ce este original în TEZAUR și ce nu

**Nu este original (marfă în 2026):** RAG cu citări; chat cu biblioteca; server MCP pentru bibliotecă (`zotero-mcp` și alții); plugin-uri pentru Obsidian/VS Code/Zotero; export RIS/BibTeX. Aici TEZAUR e *la fel de bun, în cel mai bun caz* — neevaluat.

**Combinații și piese pe care *nu le-am găsit* împreună în căutarea de azi:**
1. **Audit determinist, offline, al unui text deja scris**, legat de *pagina citată*: `[@cheie, p. 47]` → pasajul din pagina 47 al acelei surse, plus citate verbatim, cifre nepotrivite, pagină probabil corectă, citare lipsă — fără model, fără cont, utilizabil în CI (`--fail`), pe Markdown, LaTeX și LyX.
   Cel mai apropiat: Acurio (cloud, plătit, citat exact din PDF-uri încărcate). Diferența reală e **local + gratuit + scriptabil**, nu ideea. Calitatea mea, măsurată: bun pe verbatim/sursă greșită/lipsă de legătură, **slab pe parafraze și negații** — probabil sub un verificator bazat pe model precum cel descris de Acurio sau CiteCheck.
2. **Chitanțe de proveniență** (hash al întrebării, modelului, răspunsului și pasajelor, verificabile ulterior, HMAC opțional) și **declarația de utilizare AI** generată din ele (română/engleză). N-am găsit un echivalent; nu e un standard și nu e o semnătură de identitate.
3. **Mesh P2P opt-in între cercetători** (căutare federată, copierea documentelor partajate, pachete offline, WebRTC fără server). N-am găsit un instrument de cercetare local cu așa ceva. Valoarea practică e nedemonstrată (testat doar pe aceeași mașină) și există alternative banale (Syncthing, Zotero groups).
4. **Lățimea de distribuție**: același nucleu ca CLI, PortableApps, fișier HTML unic, PWA, extensie, MCP, plugin-uri pentru ~40 de aplicații, installer/uninstaller. Concurenții open-source sunt mai adânci pe o singură suprafață (Zotero, desktop). Lățimea este neobișnuită; profunzimea nu.
5. **Orientarea spre teză, în română, cu principii „trade-free”/fără cloud** — o nișă reală, dar cu piață mică.

## 3. Unde TEZAUR este în urma concurenței (fără menajamente)

| Lipsă | Cine o face |
|---|---|
| **Descoperire de literatură** (căutare în sute de milioane de lucrări, OpenAlex/Semantic Scholar) | Elicit, Consensus, Atlas, SciSpace, Research Rabbit |
| **Contextul citărilor** (cine a infirmat această lucrare) | Scite |
| **Verificarea existenței referințelor în baze externe** (Crossref/OpenAlex) | Acurio, CiteCheck, Scite — la mine lipsește; ar fi completarea naturală a `audit` |
| **Parsare bună a PDF-urilor** (tabele, ecuații, figuri, layout) | llm-for-zotero (MinerU). TEZAUR (v0.4) are doar o euristică peste stratul de text poppler: tabele cu coloane clare și ecuații pe o linie, nu fracții suprapuse, matrici, tabele-imagine sau OCR de tip Nougat/MinerU |
| **Mod agent**, scriere de note/etichete în Zotero | llm-for-zotero |
| **Interfață lustruită, utilizatori, comunitate, actualizări** | toți ceilalți; TEZAUR are un singur autor și zero utilizatori externi |
| **Calitate de răspuns demonstrată cu modele reale** | AnythingLLM etc. au măsurători (oricât de grosiere); TEZAUR nu a rulat cu un model real |
| **Scalare** (zeci de mii de pagini, embeddings, index aproximativ) | nemăsurat la mine peste 6 M de cuvinte fără embeddings |

## 4. Verdict

- **Originalitate a ideii de bază: scăzută.** Este o implementare solidă a unui model deja comun (RAG local cu citări).
- **Originalitate a *verificării și a proveniței*: moderată și reală**, dar apropiată de ce fac Acurio și lucrările academice pe același subiect; avantajul este rularea locală, gratuită și scriptabilă, iar dezavantajul calitatea euristică.
- **Originalitate a pachetului (Mesh + chitanțe + installer + atâtea suprafețe): mare ca amploare, nedovedită ca valoare.**
- **Poziționare care ar rezista**: „stratul de audit și proveniență, local, pentru biblioteca ta și pentru textul tău” — de folosit *alături de* Zotero și de `zotero-mcp`/llm-for-zotero, nu în locul lor.
- **Ce ar schimba verdictul în bine**: (1) o rulare comparativă cu un model real și un mic benchmark public; (2) verificarea existenței referințelor prin OpenAlex/Crossref (opțional, cu consimțământ, pentru că iese din „nimic nu părăsește calculatorul”); (3) un verificator de sens opțional, cu model local, pe propozițiile marcate „weak”; (4) o integrare reală cu Zotero testată pe Zotero.

## După 0.3.0: ce lipsuri s-au închis și care nu

| Lipsă (secțiunea 3) | Stare în 0.3.0 | Dovadă |
|---|---|---|
| Verificarea existenței referințelor în baze externe | **implementată** (`refs --online`: existență, an/titlu diferit, retractare; opt-in) | doar contra unor servere false; serviciile reale nu au putut fi atinse din mediul meu |
| Descoperire de literatură | **parțial**: `lit` interoghează OpenAlex/Crossref și scoate BibTeX. Nu e un motor cu rezumate, tabele de extracție sau „Consensus Meter” | idem, servere false |
| Verificator de sens pentru propoziții „weak” | **implementat** (`audit --semantic`, cu modelul tău local) | doar judecător scriptat; calitatea reală depinde de model și nu e măsurată |
| Scalare / CLI lent | **CLI 30× mai rapid** (cache BM25 pe disc) | măsurat pe 6 M de cuvinte; embeddings la scară încă nemăsurate |
| Calitate de regăsire demonstrată | **instrument** (`eval`), nu rezultat: aștept un set de întrebări reale | unit test + test CLI |
| Contextul citărilor (Scite) | **nu** | — |
| Parsare PDF avansată (MinerU) | **nu** | — |
| Mod agent, scriere în Zotero | **nu** | — |
| Interfață lustruită, comunitate, utilizatori | **nu** (a primit doar un panou de audit) | — |
| Calitate cu un model real | **tot nerulat** | — |

Concluzia rămâne cea de mai sus: poziționarea defensabilă este „stratul local de audit și proveniență alături de Zotero”, nu „cea mai bună aplicație de pe piață”.
Acest calificativ nu se poate câștiga prin cod scris de o singură persoană într-o după-amiază; se câștigă prin utilizare reală, măsurători pe documente reale și feedback de la oameni care nu sunt autorul.

## Licență
MIT, autor Alexio (https://trom.tf), ca la celelalte aplicații. Fișierul `LICENSE` este în rădăcina proiectului și în pachetele `.deb`, PyPI și npm. Notă: MIT este mai permisivă decât AGPL/GPL folosite de llm-for-zotero și Zotero RAG Assistant — oricine poate prelua codul TEZAUR, inclusiv în produse închise; asta e consecința alegerii, nu o problemă tehnică.

## Surse
- [7 Best AI Research Assistants (2026) — Atlas (publicat de un vânzător concurent)](https://www.atlasworkspace.ai/blog/best-ai-research-assistants)
- [Scite alternatives — Paperguide (vânzător concurent)](https://paperguide.ai/blog/scite-alternatives/)
- [Forum Zotero: cerere de chat AI cu biblioteca, 22–23 iun. 2026](https://forums.zotero.org/discussion/comment/514500/)
- [Zotero RAG Assistant](https://github.com/aahepburn/Zotero-RAG-Assistant)
- [llm-for-zotero](https://github.com/yilewang/llm-for-zotero)
- [zotero-mcp pe PyPI](https://pypi.org/project/zotero-mcp/)
- [AnythingLLM vs PrivateGPT vs Open WebUI — PromptQuorum](https://www.promptquorum.com/power-local-llm/anythingllm-vs-privategpt-vs-openwebui-rag)
- [Khoj — PromptQuorum](https://www.promptquorum.com/power-local-llm/khoj-ai-second-brain-review)
- [CiteCheck (alphaXiv)](https://www.alphaxiv.org/abs/2605.27700)
- [Acurio citation checker](https://acurio.ch/en/citation-checker.md)
