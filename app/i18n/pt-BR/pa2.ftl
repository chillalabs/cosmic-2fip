# pa2 — Português (Brasil)
# Tradução completa. O português de Portugal (i18n/pt) só redefine as chaves
# que mudam; o resto vem deste arquivo.

## Barra de menus
menu-file = Arquivo
menu-edit = Editar
menu-view = Exibir
new-tab = Nova aba
new-folder = Nova pasta
close-tab = Fechar aba
quit = Sair
cut = Recortar
copy = Copiar
paste = Colar
select-all = Selecionar tudo
rename-ellipsis = Renomear…
delete = Excluir
list-view = Exibição em lista
grid-view = Exibição em grade
favorites = Favoritos
settings = Configurações

## Menu de contexto
open = Abrir
open-with-ellipsis = Abrir com…
view = Visualizar
edit = Editar
calculate-size = Calcular tamanho
compress-ellipsis = Compactar…
show-details = Mostrar detalhes

## Colunas da lista
column-name = Nome
column-ext = Ext.
column-size = Tamanho
column-modified = Modificado

## Operações em andamento (painel de progresso)
op-copying = Copiando
op-moving = Movendo
op-deleting = Excluindo
op-compressing = Compactando
op-copy-failed = Falha ao copiar
op-move-failed = Falha ao mover
op-delete-failed = Falha ao excluir
op-compress-failed = Falha ao compactar
op-progress = { $operation } { $percent }%
op-files-done = { $done } de { $total } arquivos
op-bytes-done = { $done } de { $total }
op-preparing = Preparando…
close = Fechar
cancel = Cancelar

## Painéis
path-not-found = Não encontrado: { $path }
status-items = { $count ->
    [one] { $count } item
   *[other] { $count } itens
}
status-items-filtered = { $count ->
    [one] { $count } item corresponde ao filtro
   *[other] { $count } itens correspondem ao filtro
}
status-selected = { $selected } de { $total } selecionados ({ $size })
tooltip-back = Voltar (Alt+←)
tooltip-forward = Avançar (Alt+→)
tooltip-up = Pasta acima (Backspace)
tooltip-new-tab = Nova aba (Ctrl+T)
tooltip-edit-path = Editar caminho (Ctrl+L)
tooltip-cancel-esc = Cancelar (Esc)
no-tab-open = Nenhuma aba aberta
filter-placeholder = Filtrar (ex.: relatorio ou *.txt)
path-placeholder = Digite o caminho de uma pasta

## Configurações
settings-general = Geral
settings-hide-hidden = Ocultar arquivos ocultos
settings-hide-hidden-description = Arquivos e pastas cujo nome começa com um ponto
settings-separate-ext = Mostrar a extensão em uma coluna própria
settings-separate-ext-description = Exibição em lista: colunas Nome e Ext. separadas, como no Total Commander
settings-thumbnails = Mostrar miniaturas
settings-thumbnails-description = Pré-visualizações de imagens, PDFs, vídeos e fontes
settings-language = Idioma
language-system = Padrão do sistema
settings-theme = Tema
settings-color-theme = Tema de cores
settings-color-theme-description = Afeta apenas o pa2; "Sistema" segue a aparência do COSMIC
color-theme-system = Sistema
color-theme-light = Claro
color-theme-dark = Escuro
settings-icon-style = Estilo dos ícones
settings-icon-style-description = "Colorido" é igual ao app COSMIC Files
icon-style-colorful = Colorido
icon-style-monochrome = Monocromático
settings-font-size = Tamanho dos nomes de arquivo
settings-font-size-description = Um texto menor mostra mais arquivos na tela
font-size-default = Padrão ({ $px } px)
font-size-small = Pequeno ({ $px } px)
font-size-smaller = Menor ({ $px } px)
font-size-tiny = Minúsculo ({ $px } px)

## Favoritos
favorites-saved = Pastas salvas
favorites-empty = Nenhum favorito salvo ainda.
favorites-move-up = Mover para cima (Ctrl+↑)
favorites-move-down = Mover para baixo (Ctrl+↓)
favorites-add-current = Adicionar a pasta atual
favorites-add = Adicionar

## Diálogo "O arquivo já existe"
conflict-title = O arquivo já existe
conflict-body = "{ $existing }" já existe no destino. Substituir por "{ $new }"?
skip-all = Pular todos
replace-all = Substituir todos
replace = Substituir
skip = Pular

## Diálogo Compactar
compress = Compactar
compress-one = Compactar "{ $name }" em um arquivo ZIP.
compress-many = Compactar { $count } itens em um arquivo ZIP.
archive-name = Nome do arquivo compactado
default-archive-name = Arquivo

## Diálogo Nova pasta
new-folder-default-name = Nova pasta
folder-name = Nome da pasta
create = Criar

## Diálogo Renomear
rename = Renomear
renaming = Renomeando "{ $name }"
new-name = Novo nome

## Diálogo Excluir
delete-one = Mover "{ $name }" para a lixeira?
delete-many = Mover { $count } itens para a lixeira?

## Barra de teclas de função (o app adiciona "F2" etc.)
fkey-rename = Renomear
fkey-view = Visualizar
fkey-edit = Editar
fkey-copy = Copiar
fkey-move = Mover
fkey-mkdir = Nova pasta
fkey-delete = Excluir
fkey-terminal = Terminal
fkey-exit = Sair

## Diálogo Abrir com
open-with = Abrir com
open-with-one = Escolha um aplicativo para abrir "{ $name }".
open-with-many = Escolha um aplicativo para abrir { $count } itens.
loading-apps = Carregando aplicativos…
no-apps = Nenhum aplicativo encontrado.
recommended-apps = Aplicativos recomendados
other-apps = Outros aplicativos
default-app = Padrão

## Diálogo Detalhes
details-items-title = { $count } itens
details-calculating = Calculando…
details-error = Erro
details-contents = { $files } arquivos e { $folders } pastas dentro
details-size = { $size } ({ $bytes } bytes)
details-items = Itens
details-location = Local
details-total-size = Tamanho total
details-type = Tipo
details-name = Nome
details-link-target = Destino do link
details-size-label = Tamanho
details-modified = Modificado
details-accessed = Último acesso
details-created = Criado
details-permissions = Permissões
details-owner = Proprietário
details-group = Grupo
details-unknown = Desconhecido
kind-folder = Pasta
kind-file = Arquivo
kind-symlink = Link simbólico
