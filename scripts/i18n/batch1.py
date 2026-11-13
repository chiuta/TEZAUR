# keys = Romanian source text (with {placeholders}); values = (en, fr, it, es, pt, de, hu). Machine translations, not reviewed natively.
B1 = {
"Nu pot ajunge la {host}. Verifică: (1) serverul rulează; (2) permite accesul din browser (CORS): {fix}. Pagina deschisă din fișier (file://) are originea „null”, deci CORS trebuie să accepte orice origine.":(
 "Cannot reach {host}. Check: (1) the server is running; (2) allow access from the browser (CORS): {fix}. A page opened from a file (file://) has the origin “null”, so CORS must accept any origin.",
 "Impossible de joindre {host}. Vérifiez : (1) le serveur tourne ; (2) autorisez l’accès depuis le navigateur (CORS) : {fix}. Une page ouverte depuis un fichier (file://) a l’origine « null » : CORS doit donc accepter toute origine.",
 "Impossibile raggiungere {host}. Controlla: (1) il server è in esecuzione; (2) consenti l’accesso dal browser (CORS): {fix}. Una pagina aperta da file (file://) ha origine «null», quindi CORS deve accettare qualsiasi origine.",
 "No se puede acceder a {host}. Comprueba: (1) que el servidor esté en marcha; (2) permite el acceso desde el navegador (CORS): {fix}. Una página abierta desde un archivo (file://) tiene origen «null», así que CORS debe aceptar cualquier origen.",
 "Não é possível aceder a {host}. Verifique: (1) o servidor está a correr; (2) permita o acesso a partir do navegador (CORS): {fix}. Uma página aberta a partir de um ficheiro (file://) tem origem «null», por isso o CORS tem de aceitar qualquer origem.",
 "{host} ist nicht erreichbar. Prüfe: (1) der Server läuft; (2) erlaube den Zugriff aus dem Browser (CORS): {fix}. Eine aus einer Datei (file://) geöffnete Seite hat den Origin „null“, CORS muss also jeden Origin akzeptieren.",
 "Nem érhető el: {host}. Ellenőrizd: (1) fut-e a szerver; (2) engedélyezd a böngészőből való hozzáférést (CORS): {fix}. A fájlból (file://) megnyitott oldal eredete „null”, ezért a CORS-nak minden eredetet el kell fogadnia.")
,
'pornește Ollama cu variabila OLLAMA_ORIGINS="*"':(
 'start Ollama with the variable OLLAMA_ORIGINS="*"','démarrez Ollama avec la variable OLLAMA_ORIGINS="*"','avvia Ollama con la variabile OLLAMA_ORIGINS="*"','inicia Ollama con la variable OLLAMA_ORIGINS="*"','inicie o Ollama com a variável OLLAMA_ORIGINS="*"','starte Ollama mit der Variable OLLAMA_ORIGINS="*"','indítsd az Ollamát az OLLAMA_ORIGINS="*" változóval')
,
"în Jan: Settings → Local API Server → activează CORS":(
 "in Jan: Settings → Local API Server → enable CORS","dans Jan : Settings → Local API Server → activez CORS","in Jan: Settings → Local API Server → attiva CORS","en Jan: Settings → Local API Server → activa CORS","no Jan: Settings → Local API Server → ative o CORS","in Jan: Settings → Local API Server → CORS aktivieren","a Janban: Settings → Local API Server → kapcsold be a CORS-t")
,
"activează CORS în server (LM Studio: Enable CORS; llama-server: --api-key nu e necesar, dar cere antet CORS)":(
 "enable CORS in the server (LM Studio: Enable CORS; llama-server: --api-key is not needed, but it must send a CORS header)",
 "activez CORS dans le serveur (LM Studio : Enable CORS ; llama-server : --api-key n’est pas nécessaire, mais il faut un en-tête CORS)",
 "attiva CORS nel server (LM Studio: Enable CORS; llama-server: --api-key non serve, ma richiede l’intestazione CORS)",
 "activa CORS en el servidor (LM Studio: Enable CORS; llama-server: no hace falta --api-key, pero requiere la cabecera CORS)",
 "ative o CORS no servidor (LM Studio: Enable CORS; llama-server: --api-key não é necessário, mas exige o cabeçalho CORS)",
 "aktiviere CORS im Server (LM Studio: Enable CORS; llama-server: --api-key ist nicht nötig, aber ein CORS-Header ist erforderlich)",
 "kapcsold be a CORS-t a szerverben (LM Studio: Enable CORS; llama-server: az --api-key nem kell, de CORS-fejléc szükséges)")
,
"Serverul a răspuns {status}: {text}":(
 "The server answered {status}: {text}","Le serveur a répondu {status} : {text}","Il server ha risposto {status}: {text}","El servidor respondió {status}: {text}","O servidor respondeu {status}: {text}","Der Server antwortete {status}: {text}","A szerver válasza {status}: {text}")
,
"modelul a citat pasaje inexistente: {nums} — consideră afirmațiile din apropiere nesusținute":(
 "the model cited passages that do not exist: {nums} — treat the nearby claims as unsupported",
 "le modèle a cité des passages inexistants : {nums} — considérez les affirmations proches comme non étayées",
 "il modello ha citato passaggi inesistenti: {nums} — considera non supportate le affermazioni vicine",
 "el modelo citó pasajes inexistentes: {nums} — considera sin respaldo las afirmaciones cercanas",
 "o modelo citou passagens inexistentes: {nums} — considere sem suporte as afirmações próximas",
 "das Modell hat nicht vorhandene Passagen zitiert: {nums} — betrachte die Aussagen in der Nähe als unbelegt",
 "a modell nem létező szakaszokat idézett: {nums} — a közeli állításokat tekintsd alátámasztatlannak")
,
"{n} paragraf(e) lungi fără citare — verifică sau elimină":(
 "{n} long paragraph(s) without a citation — check or remove","{n} long(s) paragraphe(s) sans citation — vérifiez ou supprimez","{n} paragrafo/i lunghi senza citazione — verifica o elimina","{n} párrafo(s) largo(s) sin cita — revisa o elimina","{n} parágrafo(s) longo(s) sem citação — verifique ou elimine","{n} lange(r) Absatz/Absätze ohne Zitat — prüfen oder entfernen","{n} hosszú bekezdés idézet nélkül — ellenőrizd vagy töröld")
,
"rezum partea {i}/{n}…":("summarising part {i}/{n}…","résumé de la partie {i}/{n}…","riassumo la parte {i}/{n}…","resumiendo la parte {i}/{n}…","a resumir a parte {i}/{n}…","fasse Teil {i}/{n} zusammen…","összegzem a(z) {i}/{n}. részt…"),
"combin rezumatele ({a} → {b})…":("merging the summaries ({a} → {b})…","fusion des résumés ({a} → {b})…","unisco i riassunti ({a} → {b})…","combinando los resúmenes ({a} → {b})…","a combinar os resumos ({a} → {b})…","führe die Zusammenfassungen zusammen ({a} → {b})…","összevonom az összegzéseket ({a} → {b})…"),
"generez…":("generating…","génération…","genero…","generando…","a gerar…","generiere…","generálok…"),
"verific cu modelul…":("checking with the model…","vérification avec le modèle…","verifico con il modello…","comprobando con el modelo…","a verificar com o modelo…","prüfe mit dem Modell…","ellenőrzés a modellel…"),
"verific…":("checking…","vérification…","verifico…","comprobando…","a verificar…","prüfe…","ellenőrzés…"),
"Niciun pasaj potrivit.":("No matching passage.","Aucun passage correspondant.","Nessun passaggio corrispondente.","Ningún pasaje coincide.","Nenhuma passagem corresponde.","Keine passende Passage.","Nincs egyező szakasz."),
"potrivire slabă: cel mai bun pasaj conține sub 40% din termenii interogării — răspunsul poate fi superficial":(
 "weak match: the best passage contains under 40% of the query terms — the answer may be shallow",
 "correspondance faible : le meilleur passage contient moins de 40 % des termes de la requête — la réponse peut être superficielle",
 "corrispondenza debole: il miglior passaggio contiene meno del 40% dei termini della ricerca — la risposta può essere superficiale",
 "coincidencia débil: el mejor pasaje contiene menos del 40 % de los términos de la consulta — la respuesta puede ser superficial",
 "correspondência fraca: a melhor passagem contém menos de 40% dos termos da consulta — a resposta pode ser superficial",
 "schwache Übereinstimmung: die beste Passage enthält weniger als 40 % der Suchbegriffe — die Antwort kann oberflächlich sein",
 "gyenge találat: a legjobb szakasz a keresőszavaknak kevesebb mint 40%-át tartalmazza — a válasz felszínes lehet")
,
"{n} card(uri) eliminate: răspuns fără citare sau cu citare inexistentă":(
 "{n} card(s) removed: answer without a citation or with a non-existent one","{n} carte(s) retirée(s) : réponse sans citation ou avec une citation inexistante","{n} scheda/e rimossa/e: risposta senza citazione o con citazione inesistente","{n} tarjeta(s) eliminada(s): respuesta sin cita o con una cita inexistente","{n} cartão(ões) removido(s): resposta sem citação ou com citação inexistente","{n} Karte(n) entfernt: Antwort ohne Zitat oder mit nicht vorhandenem Zitat","{n} kártya eltávolítva: a válasz idézet nélküli vagy nem létező idézettel")
,
"oprit de utilizator":("stopped by the user","arrêté par l’utilisateur","interrotto dall’utente","detenido por el usuario","parado pelo utilizador","vom Benutzer gestoppt","a felhasználó leállította"),
"pagini citate care nu există în document: {pages}":("cited pages that do not exist in the document: {pages}","pages citées qui n’existent pas dans le document : {pages}","pagine citate che non esistono nel documento: {pages}","páginas citadas que no existen en el documento: {pages}","páginas citadas que não existem no documento: {pages}","zitierte Seiten, die im Dokument nicht existieren: {pages}","idézett oldalak, amelyek nincsenek a dokumentumban: {pages}"),
"Auditul rulează în „tezaur serve” (are acces la indexul complet și la paginile surselor). Pornește serverul și deschide această pagină de acolo.":(
 "The audit runs in “tezaur serve” (it has access to the full index and to the source pages). Start the server and open this page from there.",
 "L’audit s’exécute dans « tezaur serve » (il a accès à l’index complet et aux pages des sources). Démarrez le serveur et ouvrez cette page depuis celui-ci.",
 "L’audit funziona in «tezaur serve» (ha accesso all’indice completo e alle pagine delle fonti). Avvia il server e apri questa pagina da lì.",
 "La auditoría se ejecuta en «tezaur serve» (tiene acceso al índice completo y a las páginas de las fuentes). Inicia el servidor y abre esta página desde allí.",
 "A auditoria corre em «tezaur serve» (tem acesso ao índice completo e às páginas das fontes). Inicie o servidor e abra esta página a partir dele.",
 "Das Audit läuft in „tezaur serve“ (es hat Zugriff auf den vollständigen Index und die Quellseiten). Starte den Server und öffne diese Seite von dort.",
 "Az ellenőrzés a „tezaur serve”-ben fut (hozzáfér a teljes indexhez és a források oldalaihoz). Indítsd el a szervert, és onnan nyisd meg ezt az oldalt.")
,
"{n} locuri de deschis la sursă.":("{n} places to open at the source.","{n} endroits à ouvrir à la source.","{n} punti da aprire alla fonte.","{n} lugares para abrir en la fuente.","{n} locais para abrir na fonte.","{n} Stellen, die an der Quelle zu öffnen sind.","{n} hely, amelyet meg kell nyitni a forrásnál."),
"Nimic găsit de acest test.":("Nothing found by this test.","Rien trouvé par ce test.","Nessun problema trovato da questo test.","Este control no ha encontrado nada.","Este teste não encontrou nada.","Dieser Test hat nichts gefunden.","Ez a teszt nem talált semmit."),
"Verificarea e mecanică (suprapunere de termeni + opinia modelului {model}), nu o dovadă.":("The check is mechanical (term overlap + the opinion of model {model}), not a proof.","La vérification est mécanique (recouvrement de termes + avis du modèle {model}), pas une preuve.","La verifica è meccanica (sovrapposizione di termini + parere del modello {model}), non una prova.","La comprobación es mecánica (solapamiento de términos + opinión del modelo {model}), no una prueba.","A verificação é mecânica (sobreposição de termos + opinião do modelo {model}), não uma prova.","Die Prüfung ist mechanisch (Begriffsüberlappung + Meinung des Modells {model}), kein Beweis.","Az ellenőrzés gépies (szóegyezés + a(z) {model} modell véleménye), nem bizonyíték."),
"Verificarea e mecanică (suprapunere de termeni), nu o dovadă.":("The check is mechanical (term overlap), not a proof.","La vérification est mécanique (recouvrement de termes), pas une preuve.","La verifica è meccanica (sovrapposizione di termini), non una prova.","La comprobación es mecánica (solapamiento de términos), no una prueba.","A verificação é mecânica (sobreposição de termos), não uma prova.","Die Prüfung ist mechanisch (Begriffsüberlappung), kein Beweis.","Az ellenőrzés gépies (szóegyezés), nem bizonyíték."),
"Se citește ({i}/{n}): {name}…":("Reading ({i}/{n}): {name}…","Lecture ({i}/{n}) : {name}…","Lettura ({i}/{n}): {name}…","Leyendo ({i}/{n}): {name}…","A ler ({i}/{n}): {name}…","Lese ({i}/{n}): {name}…","Beolvasás ({i}/{n}): {name}…"),
"fără text":("no text","sans texte","senza testo","sin texto","sem texto","kein Text","nincs szöveg"),
"{d} documente · {p} pasaje":("{d} documents · {p} passages","{d} documents · {p} passages","{d} documenti · {p} passaggi","{d} documentos · {p} pasajes","{d} documentos · {p} passagens","{d} Dokumente · {p} Passagen","{d} dokumentum · {p} szakasz"),
"{n} intrări BibTeX · {m}/{d} documente asociate (cheie = nume fișier, sau câmpul file)":("{n} BibTeX entries · {m}/{d} documents matched (key = file name, or the file field)","{n} entrées BibTeX · {m}/{d} documents associés (clé = nom de fichier, ou champ file)","{n} voci BibTeX · {m}/{d} documenti associati (chiave = nome file, o campo file)","{n} entradas BibTeX · {m}/{d} documentos asociados (clave = nombre de archivo, o campo file)","{n} entradas BibTeX · {m}/{d} documentos associados (chave = nome do ficheiro, ou campo file)","{n} BibTeX-Einträge · {m}/{d} Dokumente zugeordnet (Schlüssel = Dateiname oder Feld file)","{n} BibTeX-tétel · {m}/{d} dokumentum társítva (kulcs = fájlnév vagy a file mező)"),
"Fără BibTeX: citările folosesc titlul fișierului.":("No BibTeX: citations use the file title.","Pas de BibTeX : les citations utilisent le titre du fichier.","Nessun BibTeX: le citazioni usano il titolo del file.","Sin BibTeX: las citas usan el título del archivo.","Sem BibTeX: as citações usam o título do ficheiro.","Kein BibTeX: Zitate verwenden den Dateititel.","Nincs BibTeX: az idézetek a fájl címét használják."),
"PDF fără strat de text (scanat) — rulează OCR întâi":("PDF without a text layer (scanned) — run OCR first","PDF sans couche de texte (scanné) — lancez d’abord l’OCR","PDF senza livello di testo (scansione) — esegui prima l’OCR","PDF sin capa de texto (escaneado): ejecuta primero el OCR","PDF sem camada de texto (digitalizado) — execute primeiro o OCR","PDF ohne Textebene (Scan) — zuerst OCR ausführen","PDF szövegréteg nélkül (beolvasott) — előbb futtass OCR-t"),
"format nesuportat: .{ext}":("unsupported format: .{ext}","format non pris en charge : .{ext}","formato non supportato: .{ext}","formato no admitido: .{ext}","formato não suportado: .{ext}","nicht unterstütztes Format: .{ext}","nem támogatott formátum: .{ext}"),
"arhivă zip invalidă":("invalid zip archive","archive zip invalide","archivio zip non valido","archivo zip no válido","arquivo zip inválido","ungültiges ZIP-Archiv","érvénytelen zip-archívum"),
"PDF-urile cer biblioteca pdf.js (se încarcă o singură dată de pe internet). Conectează-te sau convertește PDF-ul în .txt/.docx.":("PDFs need the pdf.js library (loaded once from the internet). Connect, or convert the PDF to .txt/.docx.","Les PDF demandent la bibliothèque pdf.js (chargée une seule fois depuis Internet). Connectez-vous ou convertissez le PDF en .txt/.docx.","I PDF richiedono la libreria pdf.js (caricata una sola volta da Internet). Connettiti o converti il PDF in .txt/.docx.","Los PDF necesitan la biblioteca pdf.js (se carga una sola vez desde Internet). Conéctate o convierte el PDF a .txt/.docx.","Os PDF precisam da biblioteca pdf.js (carregada uma só vez a partir da Internet). Ligue-se ou converta o PDF para .txt/.docx.","PDFs benötigen die Bibliothek pdf.js (wird einmalig aus dem Internet geladen). Verbinde dich oder wandle das PDF in .txt/.docx um.","A PDF-ekhez a pdf.js könyvtár kell (egyszer töltődik le az internetről). Csatlakozz, vagy alakítsd a PDF-et .txt/.docx formátumra."),
"DOCX fără document.xml":("DOCX without document.xml","DOCX sans document.xml","DOCX senza document.xml","DOCX sin document.xml","DOCX sem document.xml","DOCX ohne document.xml","DOCX document.xml nélkül"),
"server la {host} ({n} modele)":("server at {host} ({n} models)","serveur à {host} ({n} modèles)","server a {host} ({n} modelli)","servidor en {host} ({n} modelos)","servidor em {host} ({n} modelos)","Server unter {host} ({n} Modelle)","szerver itt: {host} ({n} modell)"),
"niciun model ales":("no model chosen","aucun modèle choisi","nessun modello scelto","ningún modelo elegido","nenhum modelo escolhido","kein Modell gewählt","nincs modell kiválasztva"),
"modelul „{m}” nu e în listă":("model “{m}” is not in the list","le modèle « {m} » n’est pas dans la liste","il modello «{m}» non è nell’elenco","el modelo «{m}» no está en la lista","o modelo «{m}» não está na lista","Modell „{m}“ steht nicht in der Liste","a(z) „{m}” modell nincs a listában"),
"index: {d} documente, {p} pasaje":("index: {d} documents, {p} passages","index : {d} documents, {p} passages","indice: {d} documenti, {p} passaggi","índice: {d} documentos, {p} pasajes","índice: {d} documentos, {p} passagens","Index: {d} Dokumente, {p} Passagen","index: {d} dokumentum, {p} szakasz"),
"biblioteca e goală — adaugă fișiere":("the library is empty — add files","la bibliothèque est vide — ajoutez des fichiers","la biblioteca è vuota — aggiungi file","la biblioteca está vacía: añade archivos","a biblioteca está vazia — adicione ficheiros","die Bibliothek ist leer — füge Dateien hinzu","a könyvtár üres — adj hozzá fájlokat"),
"niciun document bifat":("no document ticked","aucun document coché","nessun documento selezionato","ningún documento marcado","nenhum documento marcado","kein Dokument angehakt","nincs dokumentum kijelölve"),
}
