# 2fip — Italiano (traduzione completa)

## Barra dei menu
menu-file = File
menu-edit = Modifica
menu-view = Visualizza
new-tab = Nuova scheda
new-folder = Nuova cartella
close-tab = Chiudi scheda
quit = Esci
cut = Taglia
copy = Copia
paste = Incolla
select-all = Seleziona tutto
rename-ellipsis = Rinomina…
delete = Elimina
list-view = Vista elenco
grid-view = Vista griglia
favorites = Preferiti
settings = Impostazioni

## Menu contestuale
open = Apri
open-with-ellipsis = Apri con…
view = Visualizza
edit = Modifica
calculate-size = Calcola dimensione
compress-ellipsis = Comprimi…
show-details = Mostra dettagli

## Colonne dell'elenco
column-name = Nome
column-ext = Est.
column-size = Dimensione
column-modified = Modificato

## Operazioni in corso (pannello di avanzamento)
op-copying = Copia in corso
op-moving = Spostamento in corso
op-deleting = Eliminazione in corso
op-compressing = Compressione in corso
op-copy-failed = Copia non riuscita
op-move-failed = Spostamento non riuscito
op-delete-failed = Eliminazione non riuscita
op-compress-failed = Compressione non riuscita
op-progress = { $operation } { $percent }%
op-files-done = { $done } di { $total } file
op-bytes-done = { $done } di { $total }
op-preparing = Preparazione…
close = Chiudi
cancel = Annulla

## Pannelli
path-not-found = Non trovato: { $path }
status-items = { $count ->
    [one] 1 elemento
   *[other] { $count } elementi
}
status-items-filtered = { $count ->
    [one] 1 elemento corrisponde al filtro
   *[other] { $count } elementi corrispondono al filtro
}
status-selected = { $selected } di { $total } selezionati ({ $size })
tooltip-back = Indietro (Alt+←)
tooltip-forward = Avanti (Alt+→)
tooltip-up = Cartella superiore (Backspace)
tooltip-new-tab = Nuova scheda (Ctrl+T)
tooltip-refresh = Aggiorna entrambi i pannelli (Ctrl+R)
tooltip-edit-path = Modifica percorso (Ctrl+L)
tooltip-cancel-esc = Annulla (Esc)
no-tab-open = Nessuna scheda aperta
filter-placeholder = Filtra (es. relazione o *.txt)
path-placeholder = Digita il percorso di una cartella

## Impostazioni
settings-general = Generale
settings-show-hidden = Mostra i file nascosti
settings-show-hidden-description = File e cartelle il cui nome inizia con un punto
settings-separate-ext = Mostra l'estensione in una colonna separata
settings-separate-ext-description = Vista elenco: colonne Nome ed Est. separate, come in Total Commander
settings-thumbnails = Mostra le miniature
settings-thumbnails-description = Anteprime di immagini, PDF, video e caratteri
settings-language = Lingua
language-system = Predefinita di sistema
settings-theme = Tema
settings-color-theme = Tema dei colori
settings-color-theme-description = Riguarda solo 2fip; «Sistema» segue l'aspetto di COSMIC
color-theme-system = Sistema
color-theme-light = Chiaro
color-theme-dark = Scuro
settings-icon-style = Stile delle icone
settings-icon-style-description = «A colori» è uguale all'app COSMIC Files
icon-style-colorful = A colori
icon-style-monochrome = Monocromatico
settings-font-size = Dimensione dei nomi dei file
settings-font-size-description = Un testo più piccolo mostra più file sullo schermo
font-size-default = Predefinita ({ $px } px)
font-size-small = Piccola ({ $px } px)
font-size-smaller = Più piccola ({ $px } px)
font-size-tiny = Minuscola ({ $px } px)

## Preferiti
favorites-saved = Cartelle salvate
favorites-empty = Nessun preferito salvato.
favorites-move-up = Sposta su (Ctrl+↑)
favorites-move-down = Sposta giù (Ctrl+↓)
favorites-add-current = Aggiungi la cartella attuale
favorites-add = Aggiungi

## Finestra «Il file esiste già»
conflict-title = Il file esiste già
conflict-body = «{ $existing }» esiste già nella destinazione. Sostituirlo con «{ $new }»?
skip-all = Salta tutti
replace-all = Sostituisci tutti
replace = Sostituisci
skip = Salta

## Finestra Comprimi
compress = Comprimi
compress-one = Comprimi «{ $name }» in un archivio ZIP.
compress-many = Comprimi { $count } elementi in un archivio ZIP.
archive-name = Nome dell'archivio
default-archive-name = Archivio

## Finestra Nuova cartella
new-folder-default-name = Nuova cartella
folder-name = Nome della cartella
create = Crea

## Finestra Rinomina
rename = Rinomina
renaming = Rinomina di «{ $name }»
new-name = Nuovo nome

## Finestra Elimina
delete-one = Spostare «{ $name }» nel cestino?
delete-many = Spostare { $count } elementi nel cestino?

## Barra dei tasti funzione (l'app aggiunge «F2», ecc.)
fkey-rename = Rinomina
fkey-view = Visualizza
fkey-edit = Modifica
fkey-copy = Copia
fkey-move = Sposta
fkey-mkdir = Nuova cartella
fkey-delete = Elimina
fkey-terminal = Terminale
fkey-exit = Esci

## Finestra Apri con
open-with = Apri con
open-with-one = Scegli un'applicazione per aprire «{ $name }».
open-with-many = Scegli un'applicazione per aprire { $count } elementi.
loading-apps = Caricamento delle applicazioni…
no-apps = Nessuna applicazione trovata.
recommended-apps = Applicazioni consigliate
other-apps = Altre applicazioni
default-app = Predefinita

## Finestra Dettagli
details-items-title = { $count } elementi
details-calculating = Calcolo in corso…
details-error = Errore
details-contents = { $files } file e { $folders } cartelle all'interno
details-size = { $size } ({ $bytes } byte)
details-items = Elementi
details-location = Posizione
details-total-size = Dimensione totale
details-type = Tipo
details-name = Nome
details-link-target = Destinazione del collegamento
details-size-label = Dimensione
details-modified = Modificato
details-accessed = Ultimo accesso
details-created = Creato
details-permissions = Permessi
details-owner = Proprietario
details-group = Gruppo
details-unknown = Sconosciuto
kind-folder = Cartella
kind-file = File
kind-symlink = Collegamento simbolico

## Finestra Cerca file
find = Cerca file
tooltip-find = Cerca file (Ctrl+F)
find-in = In { $path } e nelle sue sottocartelle
find-placeholder = Nome o modello (es. relazione o *.pdf)
find-search = Cerca
find-stop = Ferma
find-hint = Digita parte di un nome e premi Invio.
find-searching = Ricerca in corso… { $count } trovati
find-results = { $count ->
    [one] 1 risultato
   *[other] { $count } risultati
}
find-no-results = Nessun risultato.
find-truncated = Vengono mostrati i primi { $count } risultati; prova un nome più specifico.
find-go-to = Vai al file

## Finestra Informazioni
about-2fip = Informazioni su 2fip
about-description = Un file manager a due pannelli, pensato per la tastiera e ispirato a Total Commander.
about-version = Versione:
about-repository = Repository:
about-license = Licenza:
