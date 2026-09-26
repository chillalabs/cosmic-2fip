# 2fip — Français (traduction complète)

## Barre de menus
menu-file = Fichier
menu-edit = Édition
menu-view = Affichage
new-tab = Nouvel onglet
new-folder = Nouveau dossier
close-tab = Fermer l’onglet
quit = Quitter
cut = Couper
copy = Copier
paste = Coller
select-all = Tout sélectionner
rename-ellipsis = Renommer…
delete = Supprimer
list-view = Vue en liste
grid-view = Vue en grille
favorites = Favoris
settings = Paramètres

## Menu contextuel
open = Ouvrir
open-with-ellipsis = Ouvrir avec…
view = Afficher
edit = Modifier
calculate-size = Calculer la taille
compress-ellipsis = Compresser…
show-details = Afficher les détails

## Colonnes de la liste
column-name = Nom
column-ext = Ext.
column-size = Taille
column-modified = Modifié

## Opérations en cours (panneau de progression)
op-copying = Copie
op-moving = Déplacement
op-deleting = Suppression
op-compressing = Compression
op-copy-failed = Échec de la copie
op-move-failed = Échec du déplacement
op-delete-failed = Échec de la suppression
op-compress-failed = Échec de la compression
op-progress = { $operation } { $percent } %
op-files-done = { $done } sur { $total } fichiers
op-bytes-done = { $done } sur { $total }
op-preparing = Préparation…
close = Fermer
cancel = Annuler

## Panneaux
path-not-found = Introuvable : { $path }
status-items = { $count ->
    [one] { $count } élément
   *[other] { $count } éléments
}
status-items-filtered = { $count ->
    [one] { $count } élément correspond au filtre
   *[other] { $count } éléments correspondent au filtre
}
status-selected = { $selected } sur { $total } sélectionnés ({ $size })
tooltip-back = Précédent (Alt+←)
tooltip-forward = Suivant (Alt+→)
tooltip-up = Dossier parent (Retour arrière)
tooltip-new-tab = Nouvel onglet (Ctrl+T)
tooltip-edit-path = Modifier le chemin (Ctrl+L)
tooltip-cancel-esc = Annuler (Échap)
no-tab-open = Aucun onglet ouvert
filter-placeholder = Filtrer (p. ex. rapport ou *.txt)
path-placeholder = Saisissez le chemin d’un dossier

## Paramètres
settings-general = Général
settings-show-hidden = Afficher les fichiers cachés
settings-show-hidden-description = Fichiers et dossiers dont le nom commence par un point
settings-separate-ext = Afficher l’extension dans sa propre colonne
settings-separate-ext-description = Vue en liste : colonnes Nom et Ext. séparées, comme dans Total Commander
settings-thumbnails = Afficher les miniatures
settings-thumbnails-description = Aperçus des images, PDF, vidéos et polices
settings-language = Langue
language-system = Langue du système
settings-theme = Thème
settings-color-theme = Thème de couleurs
settings-color-theme-description = Ne concerne que 2fip ; « Système » suit l’apparence de COSMIC
color-theme-system = Système
color-theme-light = Clair
color-theme-dark = Sombre
settings-icon-style = Style des icônes
settings-icon-style-description = « Coloré » correspond à l’application COSMIC Files
icon-style-colorful = Coloré
icon-style-monochrome = Monochrome
settings-font-size = Taille des noms de fichiers
settings-font-size-description = Un texte plus petit affiche plus de fichiers à l’écran
font-size-default = Par défaut ({ $px } px)
font-size-small = Petit ({ $px } px)
font-size-smaller = Plus petit ({ $px } px)
font-size-tiny = Minuscule ({ $px } px)

## Favoris
favorites-saved = Dossiers enregistrés
favorites-empty = Aucun favori enregistré pour l’instant.
favorites-move-up = Monter (Ctrl+↑)
favorites-move-down = Descendre (Ctrl+↓)
favorites-add-current = Ajouter le dossier actuel
favorites-add = Ajouter

## Boîte de dialogue « Le fichier existe déjà »
conflict-title = Le fichier existe déjà
conflict-body = « { $existing } » existe déjà dans la destination. Le remplacer par « { $new } » ?
skip-all = Tout ignorer
replace-all = Tout remplacer
replace = Remplacer
skip = Ignorer

## Boîte de dialogue Compresser
compress = Compresser
compress-one = Compresser « { $name } » dans une archive ZIP.
compress-many = Compresser { $count } éléments dans une archive ZIP.
archive-name = Nom de l’archive
default-archive-name = Archive

## Boîte de dialogue Nouveau dossier
new-folder-default-name = Nouveau dossier
folder-name = Nom du dossier
create = Créer

## Boîte de dialogue Renommer
rename = Renommer
renaming = Renommer « { $name } »
new-name = Nouveau nom

## Boîte de dialogue Supprimer
delete-one = Placer « { $name } » dans la corbeille ?
delete-many = Placer { $count } éléments dans la corbeille ?

## Barre des touches de fonction (l’application ajoute « F2 », etc.)
fkey-rename = Renommer
fkey-view = Afficher
fkey-edit = Modifier
fkey-copy = Copier
fkey-move = Déplacer
fkey-mkdir = Nouveau dossier
fkey-delete = Supprimer
fkey-terminal = Terminal
fkey-exit = Quitter

## Boîte de dialogue Ouvrir avec
open-with = Ouvrir avec
open-with-one = Choisissez une application pour ouvrir « { $name } ».
open-with-many = Choisissez une application pour ouvrir { $count } éléments.
loading-apps = Chargement des applications…
no-apps = Aucune application trouvée.
recommended-apps = Applications recommandées
other-apps = Autres applications
default-app = Par défaut

## Boîte de dialogue Détails
details-items-title = { $count } éléments
details-calculating = Calcul en cours…
details-error = Erreur
details-contents = { $files } fichiers et { $folders } dossiers à l’intérieur
details-size = { $size } ({ $bytes } octets)
details-items = Éléments
details-location = Emplacement
details-total-size = Taille totale
details-type = Type
details-name = Nom
details-link-target = Cible du lien
details-size-label = Taille
details-modified = Modifié
details-accessed = Dernier accès
details-created = Créé
details-permissions = Permissions
details-owner = Propriétaire
details-group = Groupe
details-unknown = Inconnu
kind-folder = Dossier
kind-file = Fichier
kind-symlink = Lien symbolique
